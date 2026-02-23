//! RSpec-style `spec!` DSL — parsing and code generation.

use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token, braced};

use crate::{
    Runtime, default_runtime, is_type_infer, ref_inner_type, slugify, wrap_async_test_body,
    wrap_test_body,
};

/// A parsed parameter from pipe syntax: `|name: &Type, name2: Type|`
pub(crate) struct PipeParam {
    pat: syn::Pat,
    ty: syn::Type,
    is_ref: bool,
}

pub(crate) enum SpecItem {
    Suite,
    Runtime(Runtime),
    Before(proc_macro2::TokenStream, Option<syn::Type>), // (body, return_type)
    After(proc_macro2::TokenStream, Vec<PipeParam>),     // (body, params)
    BeforeEach(
        proc_macro2::TokenStream,
        bool,
        Option<syn::Type>,
        Vec<PipeParam>,
    ),
    //         body                     async  return_type          input_params
    AfterEach(proc_macro2::TokenStream, bool, Vec<PipeParam>),
    //        body                      async  params
    It(String, proc_macro2::TokenStream, bool, Vec<PipeParam>),
    //         body                      async  params
    Other(proc_macro2::TokenStream),
}

pub(crate) struct SpecModule {
    vis: syn::Visibility,
    ident: Ident,
    items: Vec<SpecItem>,
}

/// Parse pipe-delimited params: `|pat: Type, pat: Type|`
/// Returns empty Vec if no pipes present.
fn parse_pipe_params(input: ParseStream) -> syn::Result<Vec<PipeParam>> {
    if !input.peek(Token![|]) {
        return Ok(Vec::new());
    }
    input.parse::<Token![|]>()?;
    let mut params = Vec::new();
    while !input.peek(Token![|]) {
        let pat: syn::Pat = syn::Pat::parse_single(input)?;
        input.parse::<Token![:]>()?;
        let ty: syn::Type = input.parse()?;
        let is_ref = matches!(&ty, syn::Type::Reference(_));
        params.push(PipeParam { pat, ty, is_ref });
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
    }
    input.parse::<Token![|]>()?;
    Ok(params)
}

/// Parse optional `-> Type` return type.
fn parse_return_type(input: ParseStream) -> syn::Result<Option<syn::Type>> {
    if input.peek(Token![->]) {
        input.parse::<Token![->]>()?;
        let ty: syn::Type = input.parse()?;
        Ok(Some(ty))
    } else {
        Ok(None)
    }
}

impl Parse for SpecModule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vis: syn::Visibility = input.parse()?;

        // Accept either `mod ident` or `describe "string literal"`
        let ident: Ident = if input.peek(Token![mod]) {
            input.parse::<Token![mod]>()?;
            input.parse()?
        } else {
            let kw: Ident = input.parse()?;
            if kw != "describe" {
                return Err(syn::Error::new(kw.span(), "expected `mod` or `describe`"));
            }
            let desc: LitStr = input.parse()?;
            let slug = slugify(&desc.value());
            Ident::new(&slug, desc.span())
        };

        let content;
        braced!(content in input);
        let mut items = Vec::new();

        while !content.is_empty() {
            // Check for `async` keyword first
            if content.peek(Token![async]) {
                let fork = content.fork();
                let _: Token![async] = fork.parse()?;

                if fork.peek(Ident) {
                    let kw: Ident = fork.parse()?;
                    match kw.to_string().as_str() {
                        "it" => {
                            let _: Token![async] = content.parse()?;
                            let _: Ident = content.parse()?;
                            let desc: LitStr = content.parse()?;
                            let params = parse_pipe_params(&content)?;
                            let body;
                            braced!(body in content);
                            items.push(SpecItem::It(desc.value(), body.parse()?, true, params));
                            continue;
                        }
                        "before_each" => {
                            let _: Token![async] = content.parse()?;
                            let _: Ident = content.parse()?;
                            let params = parse_pipe_params(&content)?;
                            let ret_ty = parse_return_type(&content)?;
                            let body;
                            braced!(body in content);
                            items.push(SpecItem::BeforeEach(body.parse()?, true, ret_ty, params));
                            continue;
                        }
                        "after_each" => {
                            let _: Token![async] = content.parse()?;
                            let _: Ident = content.parse()?;
                            let params = parse_pipe_params(&content)?;
                            let body;
                            braced!(body in content);
                            items.push(SpecItem::AfterEach(body.parse()?, true, params));
                            continue;
                        }
                        _ => {
                            // Not a known async keyword, fall through to parse as item
                        }
                    }
                }
                // Fall through: parse as regular item (e.g. `async fn helper()`)
                let item: syn::Item = content.parse()?;
                items.push(SpecItem::Other(quote! { #item }));
                continue;
            }

            if content.peek(Ident) {
                let fork = content.fork();
                let kw: Ident = fork.parse()?;
                match kw.to_string().as_str() {
                    "suite" => {
                        let _: Ident = content.parse()?;
                        content.parse::<Token![;]>()?;
                        items.push(SpecItem::Suite);
                        continue;
                    }
                    "tokio" => {
                        let _: Ident = content.parse()?;
                        content.parse::<Token![;]>()?;
                        items.push(SpecItem::Runtime(Runtime::Tokio));
                        continue;
                    }
                    "async_std" => {
                        let _: Ident = content.parse()?;
                        content.parse::<Token![;]>()?;
                        items.push(SpecItem::Runtime(Runtime::AsyncStd));
                        continue;
                    }
                    "it" => {
                        let _: Ident = content.parse()?;
                        let desc: LitStr = content.parse()?;
                        let params = parse_pipe_params(&content)?;
                        let body;
                        braced!(body in content);
                        items.push(SpecItem::It(desc.value(), body.parse()?, false, params));
                        continue;
                    }
                    "before_each" => {
                        let _: Ident = content.parse()?;
                        let params = parse_pipe_params(&content)?;
                        let ret_ty = parse_return_type(&content)?;
                        let body;
                        braced!(body in content);
                        items.push(SpecItem::BeforeEach(body.parse()?, false, ret_ty, params));
                        continue;
                    }
                    "after_each" => {
                        let _: Ident = content.parse()?;
                        let params = parse_pipe_params(&content)?;
                        let body;
                        braced!(body in content);
                        items.push(SpecItem::AfterEach(body.parse()?, false, params));
                        continue;
                    }
                    "before" => {
                        let _: Ident = content.parse()?;
                        let ret_ty = parse_return_type(&content)?;
                        if content.peek(syn::token::Brace) {
                            let body;
                            braced!(body in content);
                            items.push(SpecItem::Before(body.parse()?, ret_ty));
                            continue;
                        } else {
                            return Err(content.error("expected `{` after `before`"));
                        }
                    }
                    "after" => {
                        let _: Ident = content.parse()?;
                        let params = parse_pipe_params(&content)?;
                        if content.peek(syn::token::Brace) {
                            let body;
                            braced!(body in content);
                            items.push(SpecItem::After(body.parse()?, params));
                            continue;
                        } else {
                            return Err(content.error("expected `{` after `after`"));
                        }
                    }
                    _ => {
                        let item: syn::Item = content.parse()?;
                        items.push(SpecItem::Other(quote! { #item }));
                        continue;
                    }
                }
            }
            let item: syn::Item = content.parse()?;
            items.push(SpecItem::Other(quote! { #item }));
        }

        Ok(SpecModule { vis, ident, items })
    }
}

pub(crate) fn expand(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let parsed: SpecModule = syn::parse2(input)?;
    let mod_name = &parsed.ident;
    let vis = &parsed.vis;

    let has_suite = parsed
        .items
        .iter()
        .any(|item| matches!(item, SpecItem::Suite));

    let mut runtime: Option<Runtime> = None;
    let mut before_body: Option<proc_macro2::TokenStream> = None;
    let mut before_return_type: Option<syn::Type> = None;
    let mut after_body: Option<proc_macro2::TokenStream> = None;
    let mut after_params: Vec<PipeParam> = Vec::new();
    let mut before_each_body: Option<proc_macro2::TokenStream> = None;
    let mut before_each_is_async = false;
    let mut before_each_return_type: Option<syn::Type> = None;
    let mut before_each_params: Vec<PipeParam> = Vec::new();
    let mut after_each_body: Option<proc_macro2::TokenStream> = None;
    let mut after_each_is_async = false;
    let mut after_each_params: Vec<PipeParam> = Vec::new();
    let mut tests: Vec<(
        String,
        Ident,
        proc_macro2::TokenStream,
        bool,
        Vec<PipeParam>,
    )> = Vec::new();
    let mut other_items: Vec<proc_macro2::TokenStream> = Vec::new();

    for item in parsed.items {
        match item {
            SpecItem::Suite => {}
            SpecItem::Runtime(rt) => {
                if runtime.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "duplicate runtime specifier",
                    ));
                }
                runtime = Some(rt);
            }
            SpecItem::Before(body, ret_ty) => {
                if before_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `before` block per module",
                    ));
                }
                before_body = Some(body);
                before_return_type = ret_ty;
            }
            SpecItem::After(body, params) => {
                if after_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `after` block per module",
                    ));
                }
                after_body = Some(body);
                after_params = params;
            }
            SpecItem::BeforeEach(body, is_async, ret_ty, params) => {
                if before_each_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `before_each` block per module",
                    ));
                }
                before_each_body = Some(body);
                before_each_is_async = is_async;
                before_each_return_type = ret_ty;
                before_each_params = params;
            }
            SpecItem::AfterEach(body, is_async, params) => {
                if after_each_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `after_each` block per module",
                    ));
                }
                after_each_body = Some(body);
                after_each_is_async = is_async;
                after_each_params = params;
            }
            SpecItem::It(desc, body, is_async, params) => {
                let fn_name = format_ident!("{}", slugify(&desc));
                tests.push((desc, fn_name, body, is_async, params));
            }
            SpecItem::Other(tokens) => {
                other_items.push(tokens);
            }
        }
    }

    // Validate: async items require a runtime
    let any_async = tests.iter().any(|(_, _, _, is_async, _)| *is_async)
        || before_each_is_async
        || after_each_is_async;

    let runtime = runtime.or_else(default_runtime);

    if any_async && runtime.is_none() {
        let both_enabled = cfg!(feature = "tokio") && cfg!(feature = "async-std");
        let msg = if both_enabled {
            "both `tokio` and `async-std` features are enabled — specify the runtime \
             explicitly: add `tokio;` or `async_std;` to the module"
        } else {
            "async test cases or hooks require a runtime: add `tokio;` or `async_std;` \
             to the module, or enable the corresponding feature on `spectacular`"
        };
        return Err(syn::Error::new(proc_macro2::Span::call_site(), msg));
    }

    // Strip `-> _` from before and before_each — no longer needed as a signal
    if before_return_type.as_ref().is_some_and(is_type_infer) {
        before_return_type = None;
    }
    if before_each_return_type.as_ref().is_some_and(is_type_infer) {
        before_each_return_type = None;
    }

    // Infer before's return type from consumer &T params when not explicit
    if before_return_type.is_none() && before_body.is_some() {
        let ref_sources = after_params
            .iter()
            .chain(before_each_params.iter())
            .chain(after_each_params.iter())
            .chain(tests.iter().flat_map(|(_, _, _, _, p)| p.iter()));
        for p in ref_sources {
            if p.is_ref {
                if let Some(inner) = ref_inner_type(&p.ty) {
                    before_return_type = Some(inner);
                    break;
                }
            }
        }
    }

    let has_before = before_body.is_some();
    let has_after = after_body.is_some();
    let has_before_each = before_each_body.is_some();
    let has_after_each = after_each_body.is_some();
    let has_before_ctx = before_return_type.is_some();
    let has_before_each_ctx = before_each_return_type.is_some();

    // Detect inline mode from consumers: tests or after_each have `_`-typed params
    let has_infer_consumers = tests
        .iter()
        .any(|(_, _, _, _, params)| params.iter().any(|p| is_type_infer(&p.ty)))
        || after_each_params.iter().any(|p| is_type_infer(&p.ty));
    let before_each_needs_inline = !has_before_each_ctx && has_infer_consumers;
    let after_each_needs_inline = after_each_params.iter().any(|p| is_type_infer(&p.ty));
    let test_count = tests.len();

    // Generate before fn
    let before_fn = if let Some(body) = &before_body {
        if let Some(ref ret_ty) = before_return_type {
            Some(quote! { fn __spec_before() -> #ret_ty { #body } })
        } else {
            Some(quote! { fn __spec_before() { #body } })
        }
    } else {
        None
    };

    // Generate after fn — strip params from signature, they're passed at call site
    let after_fn = if let Some(body) = &after_body {
        if after_params.is_empty() {
            Some(quote! { fn __spec_after() { #body } })
        } else {
            let param_defs: Vec<proc_macro2::TokenStream> = after_params
                .iter()
                .map(|p| {
                    let pat = &p.pat;
                    let ty = &p.ty;
                    quote! { #pat: #ty }
                })
                .collect();
            Some(quote! { fn __spec_after(#(#param_defs),*) { #body } })
        }
    } else {
        None
    };

    // Generate before_each fn (skip when inlining)
    let before_each_fn = if before_each_needs_inline {
        None
    } else if let Some(body) = &before_each_body {
        let async_kw = if before_each_is_async {
            quote! { async }
        } else {
            quote! {}
        };
        let param_defs: Vec<proc_macro2::TokenStream> = before_each_params
            .iter()
            .map(|p| {
                let pat = &p.pat;
                let ty = &p.ty;
                quote! { #pat: #ty }
            })
            .collect();
        let ret = if let Some(ref ret_ty) = before_each_return_type {
            quote! { -> #ret_ty }
        } else {
            quote! {}
        };
        Some(quote! { #async_kw fn __spec_before_each(#(#param_defs),*) #ret { #body } })
    } else {
        None
    };

    // Generate after_each fn (skip when inlining)
    let after_each_fn = if after_each_needs_inline {
        None
    } else if let Some(body) = &after_each_body {
        let async_kw = if after_each_is_async {
            quote! { async }
        } else {
            quote! {}
        };
        let param_defs: Vec<proc_macro2::TokenStream> = after_each_params
            .iter()
            .map(|p| {
                let pat = &p.pat;
                let ty = &p.ty;
                quote! { #pat: #ty }
            })
            .collect();
        Some(quote! { #async_kw fn __spec_after_each(#(#param_defs),*) { #body } })
    } else {
        None
    };

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

    let test_fn_defs: Vec<proc_macro2::TokenStream> = tests
        .iter()
        .map(|(_desc, fn_name, body, is_async, test_params)| {
            // A test needs async if it's declared async or any hook it uses is async
            let test_needs_async = *is_async || before_each_is_async || after_each_is_async;

            let mut pre = proc_macro2::TokenStream::new();
            let mut post = proc_macro2::TokenStream::new();

            // --- Suite before ---
            if has_suite {
                pre.extend(quote! { super::__spectacular_suite::before(); });
            }

            // --- Group before ---
            if has_before {
                if has_before_ctx {
                    pre.extend(quote! {
                        let __before_ctx = __SPEC_BEFORE_CTX.get_or_init(__spec_before);
                    });
                } else {
                    pre.extend(quote! { __SPEC_BEFORE.call_once(__spec_before); });
                }
            }

            // --- Suite before_each ---
            if has_suite {
                pre.extend(quote! { super::__spectacular_suite::before_each(); });
            }

            // --- Group before_each ---
            if has_before_each {
                if before_each_needs_inline {
                    // Inline mode: bind ref params, then inline body in closure/async block
                    let ref_bindings: Vec<proc_macro2::TokenStream> = before_each_params
                        .iter()
                        .filter(|p| p.is_ref)
                        .map(|p| {
                            let pat = &p.pat;
                            let ty = &p.ty;
                            quote! { let #pat: #ty = __before_ctx; }
                        })
                        .collect();

                    let be_body = before_each_body.as_ref().unwrap();
                    let inline_expr = if before_each_is_async {
                        quote! { { #(#ref_bindings)* async move { #be_body }.await } }
                    } else {
                        quote! { { #(#ref_bindings)* (move || { #be_body })() } }
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

                    let call = if before_each_is_async {
                        quote! { __spec_before_each(#(#call_args),*).await }
                    } else {
                        quote! { __spec_before_each(#(#call_args),*) }
                    };

                    let owned_test_pats: Vec<&syn::Pat> = test_params
                        .iter()
                        .filter(|p| !p.is_ref)
                        .map(|p| &p.pat)
                        .collect();

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
                    // No return type — fire-and-forget, but may have params
                    let call_args: Vec<proc_macro2::TokenStream> = before_each_params
                        .iter()
                        .filter(|p| p.is_ref)
                        .map(|_| quote! { __before_ctx })
                        .collect();
                    if before_each_is_async {
                        if call_args.is_empty() {
                            pre.extend(quote! { __spec_before_each().await; });
                        } else {
                            pre.extend(quote! { __spec_before_each(#(#call_args),*).await; });
                        }
                    } else if call_args.is_empty() {
                        pre.extend(quote! { __spec_before_each(); });
                    } else {
                        pre.extend(quote! { __spec_before_each(#(#call_args),*); });
                    }
                }
            }

            // --- Bind ref params for test body ---
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
                body.clone()
            } else {
                quote! { #(#ref_bindings)* #body }
            };

            // --- after_each ---
            if has_after_each {
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
                    let ae_body = after_each_body.as_ref().unwrap();
                    if after_each_is_async {
                        post.extend(quote! { { #(#ref_bindings)* async { #ae_body }.await; } });
                    } else {
                        post.extend(quote! { { #(#ref_bindings)* #ae_body } });
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
                            post.extend(quote! { __spec_after_each().await; });
                        } else {
                            post.extend(quote! { __spec_after_each(#(#call_args),*).await; });
                        }
                    } else if call_args.is_empty() {
                        post.extend(quote! { __spec_after_each(); });
                    } else {
                        post.extend(quote! { __spec_after_each(#(#call_args),*); });
                    }
                }
            }
            if has_suite {
                post.extend(quote! { super::__spectacular_suite::after_each(); });
            }

            // --- after (countdown) ---
            if has_after {
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
                            __spec_after();
                        }
                    });
                } else {
                    post.extend(quote! {
                        if __SPEC_AFTER_REMAINING
                            .fetch_sub(1, ::std::sync::atomic::Ordering::SeqCst)
                            == 1
                        {
                            __spec_after(#(#call_args),*);
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
                    #test_attr
                    async fn #fn_name() {
                        #inner
                    }
                }
            } else {
                let inner = wrap_test_body(pre, body_with_bindings, post, needs_catch);

                quote! {
                    #[test]
                    fn #fn_name() {
                        #inner
                    }
                }
            }
        })
        .collect();

    Ok(quote! {
        #vis mod #mod_name {
            #(#other_items)*
            #once_static
            #countdown_static
            #before_fn
            #after_fn
            #before_each_fn
            #after_each_fn
            #(#test_fn_defs)*
        }
    })
}
