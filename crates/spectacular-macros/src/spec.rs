//! RSpec-style `spec!` DSL — parsing and code generation.

use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token, braced};

use crate::{Runtime, default_runtime, slugify, wrap_async_test_body, wrap_test_body};

pub(crate) enum SpecItem {
    Suite,
    Runtime(Runtime),
    Before(proc_macro2::TokenStream),
    After(proc_macro2::TokenStream),
    BeforeEach(proc_macro2::TokenStream, bool),  // (body, is_async)
    AfterEach(proc_macro2::TokenStream, bool),   // (body, is_async)
    It(String, proc_macro2::TokenStream, bool),  // (desc, body, is_async)
    Other(proc_macro2::TokenStream),
}

pub(crate) struct SpecModule {
    vis: syn::Visibility,
    ident: Ident,
    items: Vec<SpecItem>,
}

impl Parse for SpecModule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vis: syn::Visibility = input.parse()?;
        input.parse::<Token![mod]>()?;
        let ident: Ident = input.parse()?;
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
                            let body;
                            braced!(body in content);
                            items.push(SpecItem::It(desc.value(), body.parse()?, true));
                            continue;
                        }
                        "before_each" => {
                            let _: Token![async] = content.parse()?;
                            let _: Ident = content.parse()?;
                            let body;
                            braced!(body in content);
                            items.push(SpecItem::BeforeEach(body.parse()?, true));
                            continue;
                        }
                        "after_each" => {
                            let _: Token![async] = content.parse()?;
                            let _: Ident = content.parse()?;
                            let body;
                            braced!(body in content);
                            items.push(SpecItem::AfterEach(body.parse()?, true));
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
                        let body;
                        braced!(body in content);
                        items.push(SpecItem::It(desc.value(), body.parse()?, false));
                        continue;
                    }
                    "before_each" => {
                        let _: Ident = content.parse()?;
                        let body;
                        braced!(body in content);
                        items.push(SpecItem::BeforeEach(body.parse()?, false));
                        continue;
                    }
                    "after_each" => {
                        let _: Ident = content.parse()?;
                        let body;
                        braced!(body in content);
                        items.push(SpecItem::AfterEach(body.parse()?, false));
                        continue;
                    }
                    "before" => {
                        let _: Ident = content.parse()?;
                        if content.peek(syn::token::Brace) {
                            let body;
                            braced!(body in content);
                            items.push(SpecItem::Before(body.parse()?));
                            continue;
                        } else {
                            return Err(content.error("expected `{` after `before`"));
                        }
                    }
                    "after" => {
                        let _: Ident = content.parse()?;
                        if content.peek(syn::token::Brace) {
                            let body;
                            braced!(body in content);
                            items.push(SpecItem::After(body.parse()?));
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
    let mut after_body: Option<proc_macro2::TokenStream> = None;
    let mut before_each_body: Option<proc_macro2::TokenStream> = None;
    let mut before_each_is_async = false;
    let mut after_each_body: Option<proc_macro2::TokenStream> = None;
    let mut after_each_is_async = false;
    let mut tests: Vec<(String, Ident, proc_macro2::TokenStream, bool)> = Vec::new();
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
            SpecItem::Before(body) => {
                if before_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `before` block per module",
                    ));
                }
                before_body = Some(body);
            }
            SpecItem::After(body) => {
                if after_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `after` block per module",
                    ));
                }
                after_body = Some(body);
            }
            SpecItem::BeforeEach(body, is_async) => {
                if before_each_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `before_each` block per module",
                    ));
                }
                before_each_body = Some(body);
                before_each_is_async = is_async;
            }
            SpecItem::AfterEach(body, is_async) => {
                if after_each_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `after_each` block per module",
                    ));
                }
                after_each_body = Some(body);
                after_each_is_async = is_async;
            }
            SpecItem::It(desc, body, is_async) => {
                let fn_name = format_ident!("{}", slugify(&desc));
                tests.push((desc, fn_name, body, is_async));
            }
            SpecItem::Other(tokens) => {
                other_items.push(tokens);
            }
        }
    }

    // Validate: async items require a runtime
    let any_async = tests.iter().any(|(_, _, _, is_async)| *is_async)
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

    let has_before = before_body.is_some();
    let has_after = after_body.is_some();
    let has_before_each = before_each_body.is_some();
    let has_after_each = after_each_body.is_some();
    let test_count = tests.len();

    let before_fn = before_body.map(|body| quote! { fn __spec_before() { #body } });
    let after_fn = after_body.map(|body| quote! { fn __spec_after() { #body } });
    let before_each_fn = if let Some(body) = before_each_body {
        if before_each_is_async {
            Some(quote! { async fn __spec_before_each() { #body } })
        } else {
            Some(quote! { fn __spec_before_each() { #body } })
        }
    } else {
        None
    };
    let after_each_fn = if let Some(body) = after_each_body {
        if after_each_is_async {
            Some(quote! { async fn __spec_after_each() { #body } })
        } else {
            Some(quote! { fn __spec_after_each() { #body } })
        }
    } else {
        None
    };

    let once_static = has_before.then(|| {
        quote! { static __SPEC_BEFORE: ::std::sync::Once = ::std::sync::Once::new(); }
    });

    let countdown_static = has_after.then(|| {
        quote! {
            static __SPEC_AFTER_REMAINING: ::std::sync::atomic::AtomicUsize =
                ::std::sync::atomic::AtomicUsize::new(#test_count);
        }
    });

    let test_fn_defs: Vec<proc_macro2::TokenStream> = tests
        .iter()
        .map(|(_desc, fn_name, body, is_async)| {
            // A test needs async if it's declared async or any hook it uses is async
            let test_needs_async = *is_async || before_each_is_async || after_each_is_async;

            let mut pre = proc_macro2::TokenStream::new();
            let mut post = proc_macro2::TokenStream::new();

            if has_suite {
                pre.extend(quote! { super::__spectacular_suite::before(); });
            }
            if has_before {
                pre.extend(quote! { __SPEC_BEFORE.call_once(__spec_before); });
            }
            if has_suite {
                pre.extend(quote! { super::__spectacular_suite::before_each(); });
            }
            if has_before_each {
                if before_each_is_async {
                    pre.extend(quote! { __spec_before_each().await; });
                } else {
                    pre.extend(quote! { __spec_before_each(); });
                }
            }

            if has_after_each {
                if after_each_is_async {
                    post.extend(quote! { __spec_after_each().await; });
                } else {
                    post.extend(quote! { __spec_after_each(); });
                }
            }
            if has_suite {
                post.extend(quote! { super::__spectacular_suite::after_each(); });
            }
            if has_after {
                post.extend(quote! {
                    if __SPEC_AFTER_REMAINING
                        .fetch_sub(1, ::std::sync::atomic::Ordering::SeqCst)
                        == 1
                    {
                        __spec_after();
                    }
                });
            }

            let needs_catch = has_after || has_after_each || has_suite;

            if test_needs_async {
                let rt = runtime.unwrap();
                let test_attr = rt.test_attr();
                let inner = wrap_async_test_body(pre, body.clone(), post, needs_catch);

                quote! {
                    #test_attr
                    async fn #fn_name() {
                        #inner
                    }
                }
            } else {
                let inner = wrap_test_body(pre, body.clone(), post, needs_catch);

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
