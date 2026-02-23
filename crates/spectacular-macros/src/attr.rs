//! Attribute-style `#[test_suite]` — parsing and code generation.

use quote::quote;
use syn::{ItemFn, ItemMod};

use crate::{
    Runtime, default_runtime, is_type_infer, ref_inner_type, wrap_async_test_body, wrap_test_body,
};

/// Extract a meaningful return type from a function signature.
/// Returns `None` for `()`, default return, or empty tuple.
fn extract_return_type(func: &ItemFn) -> Option<syn::Type> {
    match &func.sig.output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, ty) => {
            // Filter out `()` returns
            if let syn::Type::Tuple(t) = ty.as_ref()
                && t.elems.is_empty()
            {
                return None;
            }
            Some(ty.as_ref().clone())
        }
    }
}

/// A parameter extracted from a function signature.
struct Param {
    pat: syn::Pat,
    ty: syn::Type,
    is_ref: bool,
}

/// Extract typed parameters from a function's signature.
fn extract_params(func: &ItemFn) -> Vec<Param> {
    func.sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                let is_ref = matches!(pat_type.ty.as_ref(), syn::Type::Reference(_));
                Some(Param {
                    pat: pat_type.pat.as_ref().clone(),
                    ty: pat_type.ty.as_ref().clone(),
                    is_ref,
                })
            } else {
                None
            }
        })
        .collect()
}

pub(crate) fn expand(
    input: ItemMod,
    has_suite: bool,
    runtime: Option<Runtime>,
) -> syn::Result<proc_macro2::TokenStream> {
    let mod_name = &input.ident;
    let vis = &input.vis;

    let Some((_, items)) = &input.content else {
        return Err(syn::Error::new_spanned(
            &input,
            "#[test_suite] requires an inline module (not `mod foo;`)",
        ));
    };

    let mut before_fn: Option<&ItemFn> = None;
    let mut after_fn: Option<&ItemFn> = None;
    let mut before_each_fn: Option<&ItemFn> = None;
    let mut after_each_fn: Option<&ItemFn> = None;
    let mut test_fns: Vec<&ItemFn> = Vec::new();
    let mut other_items: Vec<&syn::Item> = Vec::new();

    for item in items {
        match item {
            syn::Item::Fn(func) => {
                let mut is_test = false;
                for attr in &func.attrs {
                    if attr.path().is_ident("before") {
                        if before_fn.is_some() {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "only one #[before] per module",
                            ));
                        }
                        before_fn = Some(func);
                    } else if attr.path().is_ident("after") {
                        if after_fn.is_some() {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "only one #[after] per module",
                            ));
                        }
                        after_fn = Some(func);
                    } else if attr.path().is_ident("before_each") {
                        if before_each_fn.is_some() {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "only one #[before_each] per module",
                            ));
                        }
                        before_each_fn = Some(func);
                    } else if attr.path().is_ident("after_each") {
                        if after_each_fn.is_some() {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "only one #[after_each] per module",
                            ));
                        }
                        after_each_fn = Some(func);
                    } else if attr.path().is_ident("test") {
                        is_test = true;
                    }
                }
                if is_test {
                    test_fns.push(func);
                } else {
                    other_items.push(item);
                }
            }
            _ => other_items.push(item),
        }
    }

    // Validate: async tests/hooks require a runtime
    let any_async = test_fns.iter().any(|f| f.sig.asyncness.is_some())
        || before_each_fn.is_some_and(|f| f.sig.asyncness.is_some())
        || after_each_fn.is_some_and(|f| f.sig.asyncness.is_some());

    let runtime = runtime.or_else(default_runtime);

    if any_async && runtime.is_none() {
        let both_enabled = cfg!(feature = "tokio") && cfg!(feature = "async-std");
        let msg = if both_enabled {
            "both `tokio` and `async-std` features are enabled — specify the runtime \
             explicitly: #[test_suite(tokio)] or #[test_suite(async_std)]"
        } else {
            "async test cases or hooks require a runtime: use #[test_suite(tokio)] or \
             #[test_suite(async_std)], or enable the corresponding feature on `spectacular`"
        };
        return Err(syn::Error::new_spanned(&input.ident, msg));
    }

    // Validate: before/after (run-once) must be sync
    if let Some(f) = before_fn
        && f.sig.asyncness.is_some()
    {
        return Err(syn::Error::new_spanned(
            &f.sig,
            "#[before] hooks must be sync (async run-once hooks are not yet supported)",
        ));
    }
    if let Some(f) = after_fn
        && f.sig.asyncness.is_some()
    {
        return Err(syn::Error::new_spanned(
            &f.sig,
            "#[after] hooks must be sync (async run-once hooks are not yet supported)",
        ));
    }

    // --- Context analysis ---
    let mut before_return_type = before_fn.and_then(extract_return_type);
    let before_each_return_type = before_each_fn.and_then(extract_return_type);

    // Strip `-> _` from before and before_each — no longer needed as a signal
    if before_return_type.as_ref().is_some_and(is_type_infer) {
        before_return_type = None;
    }
    let before_each_return_type = before_each_return_type.filter(|ty| !is_type_infer(ty));

    let before_name = before_fn.map(|f| &f.sig.ident);
    let after_name = after_fn.map(|f| &f.sig.ident);
    let before_each_name = before_each_fn.map(|f| &f.sig.ident);
    let after_each_name = after_each_fn.map(|f| &f.sig.ident);
    let before_each_is_async = before_each_fn.is_some_and(|f| f.sig.asyncness.is_some());
    let after_each_is_async = after_each_fn.is_some_and(|f| f.sig.asyncness.is_some());

    let has_before = before_name.is_some();
    let has_after = after_name.is_some();
    let has_after_each = after_each_name.is_some();
    let test_count = test_fns.len();

    // Extract params from before_each (ref params come from before context)
    let before_each_params = before_each_fn.map(extract_params).unwrap_or_default();
    let _before_each_has_before_ref = before_each_params.iter().any(|p| p.is_ref);

    // Extract params from after_each
    let after_each_params = after_each_fn.map(extract_params).unwrap_or_default();
    let after_each_needs_inline = after_each_params.iter().any(|p| is_type_infer(&p.ty));

    // Extract params from after
    let after_params = after_fn.map(extract_params).unwrap_or_default();

    // Infer before's return type from consumer &T params when not explicit
    let before_type_was_inferred;
    if before_return_type.is_none() && has_before {
        let find_ref = |params: &[Param]| -> Option<syn::Type> {
            params
                .iter()
                .filter(|p| p.is_ref)
                .find_map(|p| ref_inner_type(&p.ty))
        };
        before_return_type = find_ref(&after_params)
            .or_else(|| find_ref(&before_each_params))
            .or_else(|| find_ref(&after_each_params))
            .or_else(|| test_fns.iter().find_map(|f| find_ref(&extract_params(f))));
        before_type_was_inferred = before_return_type.is_some();
    } else {
        before_type_was_inferred = false;
    }

    let has_before_ctx = before_return_type.is_some();
    let has_before_each_ctx = before_each_return_type.is_some();

    // Detect inline mode from consumers: tests or after_each have `_`-typed params
    let has_infer_consumers = test_fns
        .iter()
        .any(|f| extract_params(f).iter().any(|p| is_type_infer(&p.ty)))
        || after_each_needs_inline;
    let before_each_needs_inline = !has_before_each_ctx && has_infer_consumers;

    // Extract before_each body for inlining
    let before_each_block = before_each_fn.map(|f| &f.block);

    let cleaned_items: Vec<proc_macro2::TokenStream> = other_items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Fn(func) = item {
                // Skip before_each/after_each functions when inlining
                let is_before_each = func.attrs.iter().any(|a| a.path().is_ident("before_each"));
                let is_after_each = func.attrs.iter().any(|a| a.path().is_ident("after_each"));
                if (before_each_needs_inline && is_before_each)
                    || (after_each_needs_inline && is_after_each)
                {
                    return None;
                }
                let mut clean = func.clone();
                // Strip hook attributes
                clean.attrs.retain(|a| {
                    !a.path().is_ident("before")
                        && !a.path().is_ident("after")
                        && !a.path().is_ident("before_each")
                        && !a.path().is_ident("after_each")
                });
                // Add inferred return type to before function
                let is_before = func.attrs.iter().any(|a| a.path().is_ident("before"));
                if is_before && before_type_was_inferred {
                    if let Some(ref ret_ty) = before_return_type {
                        clean.sig.output =
                            syn::ReturnType::Type(Default::default(), Box::new(ret_ty.clone()));
                    }
                }
                Some(quote! { #clean })
            } else {
                Some(quote! { #item })
            }
        })
        .collect();

    // Static for before: OnceLock<T> if returns context, else Once
    let once_static = if has_before {
        if let Some(ref ret_ty) = before_return_type {
            Some(quote! {
                static __SPEC_BEFORE_CTX: ::std::sync::OnceLock<#ret_ty> =
                    ::std::sync::OnceLock::new();
            })
        } else {
            Some(quote! {
                static __SPEC_BEFORE: ::std::sync::Once = ::std::sync::Once::new();
            })
        }
    } else {
        None
    };

    let countdown_static = has_after.then(|| {
        quote! {
            static __SPEC_AFTER_REMAINING: ::std::sync::atomic::AtomicUsize =
                ::std::sync::atomic::AtomicUsize::new(#test_count);
        }
    });

    let test_fn_defs: Vec<proc_macro2::TokenStream> = test_fns
        .iter()
        .map(|func| {
            let fn_name = &func.sig.ident;
            let fn_vis = &func.vis;
            let body = &func.block;
            let is_async = func.sig.asyncness.is_some();
            let other_attrs: Vec<_> = func
                .attrs
                .iter()
                .filter(|a| !a.path().is_ident("test"))
                .collect();

            // Extract test params
            let test_params = extract_params(func);

            // Determine if this specific test needs async wrapping
            let test_needs_async = is_async || (before_each_is_async || after_each_is_async);

            let mut pre = proc_macro2::TokenStream::new();
            let mut post = proc_macro2::TokenStream::new();

            // --- Suite before ---
            if has_suite {
                pre.extend(quote! { super::__spectacular_suite::before(); });
            }

            // --- Group before ---
            if has_before {
                if has_before_ctx {
                    let name = before_name.unwrap();
                    pre.extend(quote! {
                        let __before_ctx = __SPEC_BEFORE_CTX.get_or_init(#name);
                    });
                } else {
                    let name = before_name.unwrap();
                    pre.extend(quote! { __SPEC_BEFORE.call_once(#name); });
                }
            }

            // --- Suite before_each ---
            if has_suite {
                pre.extend(quote! { super::__spectacular_suite::before_each(); });
            }

            // --- Group before_each ---
            if let Some(name) = before_each_name {
                if before_each_needs_inline {
                    // Inline mode: bind ref params, inline body in closure/async block
                    let ref_bindings: Vec<proc_macro2::TokenStream> = before_each_params
                        .iter()
                        .filter(|p| p.is_ref)
                        .map(|p| {
                            let pat = &p.pat;
                            let ty = &p.ty;
                            quote! { let #pat: #ty = __before_ctx; }
                        })
                        .collect();

                    let be_body = before_each_block.unwrap();
                    let be_stmts = &be_body.stmts;
                    let inline_expr = if before_each_is_async {
                        quote! { { #(#ref_bindings)* async move { #(#be_stmts)* }.await } }
                    } else {
                        quote! { { #(#ref_bindings)* (move || { #(#be_stmts)* })() } }
                    };

                    // Destructure into owned params (same logic as function-call path)
                    let owned_test_pats: Vec<&syn::Pat> = test_params
                        .iter()
                        .filter(|p| !p.is_ref)
                        .map(|p| &p.pat)
                        .collect();

                    if owned_test_pats.len() > 1 {
                        pre.extend(quote! {
                            let (#(#owned_test_pats),*) = #inline_expr;
                        });
                    } else if owned_test_pats.len() == 1 {
                        let pat = owned_test_pats[0];
                        pre.extend(quote! {
                            let #pat = #inline_expr;
                        });
                    } else {
                        let after_each_owned_pats: Vec<&syn::Pat> = after_each_params
                            .iter()
                            .filter(|p| !p.is_ref)
                            .map(|p| &p.pat)
                            .collect();

                        if after_each_owned_pats.len() > 1 {
                            pre.extend(quote! {
                                let (#(#after_each_owned_pats),*) = #inline_expr;
                            });
                        } else if after_each_owned_pats.len() == 1 {
                            let pat = after_each_owned_pats[0];
                            pre.extend(quote! {
                                let #pat = #inline_expr;
                            });
                        } else {
                            pre.extend(quote! { #inline_expr; });
                        }
                    }
                } else if has_before_each_ctx {
                    // Function call mode
                    let call_args: Vec<proc_macro2::TokenStream> = before_each_params
                        .iter()
                        .filter(|p| p.is_ref)
                        .map(|_| quote! { __before_ctx })
                        .collect();

                    let owned_test_pats: Vec<&syn::Pat> = test_params
                        .iter()
                        .filter(|p| !p.is_ref)
                        .map(|p| &p.pat)
                        .collect();

                    let call = if before_each_is_async {
                        quote! { #name(#(#call_args),*).await }
                    } else {
                        quote! { #name(#(#call_args),*) }
                    };

                    if owned_test_pats.len() > 1 {
                        pre.extend(quote! {
                            let (#(#owned_test_pats),*) = #call;
                        });
                    } else if owned_test_pats.len() == 1 {
                        let pat = owned_test_pats[0];
                        pre.extend(quote! {
                            let #pat = #call;
                        });
                    } else {
                        let after_each_owned_pats: Vec<&syn::Pat> = after_each_params
                            .iter()
                            .filter(|p| !p.is_ref)
                            .map(|p| &p.pat)
                            .collect();

                        if after_each_owned_pats.len() > 1 {
                            pre.extend(quote! {
                                let (#(#after_each_owned_pats),*) = #call;
                            });
                        } else if after_each_owned_pats.len() == 1 {
                            let pat = after_each_owned_pats[0];
                            pre.extend(quote! {
                                let #pat = #call;
                            });
                        } else {
                            pre.extend(quote! { #call; });
                        }
                    }
                } else {
                    // No return type — fire-and-forget, but may have before ctx param
                    let call_args: Vec<proc_macro2::TokenStream> = before_each_params
                        .iter()
                        .filter(|p| p.is_ref)
                        .map(|_| quote! { __before_ctx })
                        .collect();
                    if before_each_is_async {
                        if call_args.is_empty() {
                            pre.extend(quote! { #name().await; });
                        } else {
                            pre.extend(quote! { #name(#(#call_args),*).await; });
                        }
                    } else if call_args.is_empty() {
                        pre.extend(quote! { #name(); });
                    } else {
                        pre.extend(quote! { #name(#(#call_args),*); });
                    }
                }
            }

            // --- Bind ref params for test body ---
            // Tests with ref params get them from __before_ctx
            let ref_bindings: Vec<proc_macro2::TokenStream> = test_params
                .iter()
                .filter(|p| p.is_ref)
                .map(|p| {
                    let pat = &p.pat;
                    let ty = &p.ty;
                    quote! { let #pat: #ty = __before_ctx; }
                })
                .collect();

            let body_with_bindings = if ref_bindings.is_empty() {
                quote! { #body }
            } else {
                // We need to inline bindings before the body statements
                let stmts = &func.block.stmts;
                quote! {
                    { #(#ref_bindings)* #(#stmts)* }
                }
            };

            // --- after_each ---
            if let Some(name) = after_each_name {
                if after_each_needs_inline {
                    // Inline mode: bind params, inline body directly
                    let ref_bindings: Vec<proc_macro2::TokenStream> = after_each_params
                        .iter()
                        .filter(|p| p.is_ref)
                        .map(|p| {
                            let pat = &p.pat;
                            let ty = &p.ty;
                            quote! { let #pat: #ty = __before_ctx; }
                        })
                        .collect();
                    let ae_stmts = &after_each_fn.unwrap().block.stmts;
                    if after_each_is_async {
                        post.extend(
                            quote! { { #(#ref_bindings)* async { #(#ae_stmts)* }.await; } },
                        );
                    } else {
                        post.extend(quote! { { #(#ref_bindings)* #(#ae_stmts)* } });
                    }
                } else {
                    let call_args: Vec<proc_macro2::TokenStream> = after_each_params
                        .iter()
                        .map(|p| {
                            if p.is_ref {
                                quote! { __before_ctx }
                            } else {
                                let pat = &p.pat;
                                quote! { #pat }
                            }
                        })
                        .collect();

                    if after_each_is_async {
                        if call_args.is_empty() {
                            post.extend(quote! { #name().await; });
                        } else {
                            post.extend(quote! { #name(#(#call_args),*).await; });
                        }
                    } else if call_args.is_empty() {
                        post.extend(quote! { #name(); });
                    } else {
                        post.extend(quote! { #name(#(#call_args),*); });
                    }
                }
            }
            if has_suite {
                post.extend(quote! { super::__spectacular_suite::after_each(); });
            }

            // --- after (countdown) ---
            if let Some(name) = after_name {
                let call_args: Vec<proc_macro2::TokenStream> = after_params
                    .iter()
                    .map(|p| {
                        if p.is_ref {
                            quote! { __before_ctx }
                        } else {
                            let pat = &p.pat;
                            quote! { #pat }
                        }
                    })
                    .collect();

                if call_args.is_empty() {
                    post.extend(quote! {
                        if __SPEC_AFTER_REMAINING
                            .fetch_sub(1, ::std::sync::atomic::Ordering::SeqCst)
                            == 1
                        {
                            #name();
                        }
                    });
                } else {
                    post.extend(quote! {
                        if __SPEC_AFTER_REMAINING
                            .fetch_sub(1, ::std::sync::atomic::Ordering::SeqCst)
                            == 1
                        {
                            #name(#(#call_args),*);
                        }
                    });
                }
            }

            let needs_catch = has_after || has_after_each || has_suite;

            if test_needs_async {
                let rt = runtime.unwrap();
                let test_attr = rt.test_attr();
                let inner = wrap_async_test_body(pre, body_with_bindings, post, needs_catch);

                quote! {
                    #(#other_attrs)*
                    #test_attr
                    #fn_vis async fn #fn_name() {
                        #inner
                    }
                }
            } else {
                let inner = wrap_test_body(pre, body_with_bindings, post, needs_catch);

                quote! {
                    #(#other_attrs)*
                    #[test]
                    #fn_vis fn #fn_name() {
                        #inner
                    }
                }
            }
        })
        .collect();

    Ok(quote! {
        #vis mod #mod_name {
            #(#cleaned_items)*
            #once_static
            #countdown_static
            #(#test_fn_defs)*
        }
    })
}
