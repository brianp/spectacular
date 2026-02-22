//! Attribute-style `#[test_suite]` — parsing and code generation.

use quote::quote;
use syn::{ItemFn, ItemMod};

use crate::{Runtime, default_runtime, wrap_async_test_body, wrap_test_body};

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

    let cleaned_items: Vec<proc_macro2::TokenStream> = other_items
        .iter()
        .map(|item| {
            if let syn::Item::Fn(func) = item {
                let mut clean = func.clone();
                clean.attrs.retain(|a| {
                    !a.path().is_ident("before")
                        && !a.path().is_ident("after")
                        && !a.path().is_ident("before_each")
                        && !a.path().is_ident("after_each")
                });
                quote! { #clean }
            } else {
                quote! { #item }
            }
        })
        .collect();

    let once_static = has_before.then(|| {
        quote! { static __SPEC_BEFORE: ::std::sync::Once = ::std::sync::Once::new(); }
    });

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

            // Determine if this specific test needs async wrapping
            // A test is async if it's declared async, or if any hook it calls is async
            let test_needs_async = is_async || (before_each_is_async || after_each_is_async);

            let mut pre = proc_macro2::TokenStream::new();
            let mut post = proc_macro2::TokenStream::new();

            if has_suite {
                pre.extend(quote! { super::__spectacular_suite::before(); });
            }
            if let Some(name) = before_name {
                pre.extend(quote! { __SPEC_BEFORE.call_once(#name); });
            }
            if has_suite {
                pre.extend(quote! { super::__spectacular_suite::before_each(); });
            }
            if let Some(name) = before_each_name {
                if before_each_is_async {
                    pre.extend(quote! { #name().await; });
                } else {
                    pre.extend(quote! { #name(); });
                }
            }

            if let Some(name) = after_each_name {
                if after_each_is_async {
                    post.extend(quote! { #name().await; });
                } else {
                    post.extend(quote! { #name(); });
                }
            }
            if has_suite {
                post.extend(quote! { super::__spectacular_suite::after_each(); });
            }
            if let Some(name) = after_name {
                post.extend(quote! {
                    if __SPEC_AFTER_REMAINING
                        .fetch_sub(1, ::std::sync::atomic::Ordering::SeqCst)
                        == 1
                    {
                        #name();
                    }
                });
            }

            let needs_catch = has_after || has_after_each || has_suite;

            if test_needs_async {
                let rt = runtime.unwrap();
                let test_attr = rt.test_attr();
                let body_tokens = if is_async {
                    quote! { #body }
                } else {
                    // Sync test body in async context — just inline it
                    quote! { #body }
                };
                let inner = wrap_async_test_body(pre, body_tokens, post, needs_catch);

                quote! {
                    #(#other_attrs)*
                    #test_attr
                    #fn_vis async fn #fn_name() {
                        #inner
                    }
                }
            } else {
                let inner = wrap_test_body(pre, quote! { #body }, post, needs_catch);

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
