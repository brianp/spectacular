//! Proc macros for the [spectacular](https://docs.rs/spectacular) test framework.
//!
//! This crate provides the macro implementations. Use the `spectacular` crate
//! directly for the public API and documentation.

mod attr;
mod spec;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemMod, braced};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Runtime {
    Tokio,
    AsyncStd,
}

impl Runtime {
    pub(crate) fn test_attr(&self) -> proc_macro2::TokenStream {
        match self {
            Runtime::Tokio => quote! { #[tokio::test] },
            Runtime::AsyncStd => quote! { #[async_std::test] },
        }
    }
}

/// Returns the default async runtime when exactly one runtime feature is enabled.
///
/// - `tokio` feature only → `Some(Runtime::Tokio)`
/// - `async-std` feature only → `Some(Runtime::AsyncStd)`
/// - both or neither → `None`
pub(crate) fn default_runtime() -> Option<Runtime> {
    let tokio = cfg!(feature = "tokio");
    let async_std = cfg!(feature = "async-std");
    match (tokio, async_std) {
        (true, false) => Some(Runtime::Tokio),
        (false, true) => Some(Runtime::AsyncStd),
        _ => None,
    }
}

pub(crate) fn is_type_infer(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Infer(_))
}

pub(crate) fn slugify(s: &str) -> String {
    let mut result = String::new();
    let mut prev_underscore = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c.to_ascii_lowercase());
            prev_underscore = false;
        } else if !prev_underscore {
            result.push('_');
            prev_underscore = true;
        }
    }
    let trimmed_len = result.trim_end_matches('_').len();
    result.truncate(trimmed_len);
    if result.is_empty() {
        return "test".to_string();
    }
    if result.starts_with(|c: char| c.is_ascii_digit()) {
        result.insert(0, '_');
    }
    result
}

pub(crate) fn wrap_test_body(
    pre: proc_macro2::TokenStream,
    body: proc_macro2::TokenStream,
    post: proc_macro2::TokenStream,
    needs_catch: bool,
) -> proc_macro2::TokenStream {
    if needs_catch {
        quote! {
            #pre
            let __spectacular_result = ::std::panic::catch_unwind(
                ::std::panic::AssertUnwindSafe(|| { #body })
            );
            #post
            if let ::std::result::Result::Err(__e) = __spectacular_result {
                ::std::panic::resume_unwind(__e);
            }
        }
    } else {
        quote! {
            #pre
            #body
        }
    }
}

pub(crate) fn wrap_async_test_body(
    pre: proc_macro2::TokenStream,
    body: proc_macro2::TokenStream,
    post: proc_macro2::TokenStream,
    needs_catch: bool,
) -> proc_macro2::TokenStream {
    if needs_catch {
        quote! {
            #pre
            let __spectacular_result = ::spectacular::__internal::catch_unwind_future(
                async { #body }
            ).await;
            #post
            if let ::std::result::Result::Err(__e) = __spectacular_result {
                ::std::panic::resume_unwind(__e);
            }
        }
    } else {
        quote! {
            #pre
            #body
        }
    }
}

struct SuiteBlock {
    before: Option<proc_macro2::TokenStream>,
    before_each: Option<proc_macro2::TokenStream>,
    after_each: Option<proc_macro2::TokenStream>,
}

impl Parse for SuiteBlock {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut before = None;
        let mut before_each = None;
        let mut after_each = None;

        while !input.is_empty() {
            let kw: Ident = input.parse()?;
            match kw.to_string().as_str() {
                "before" => {
                    if before.is_some() {
                        return Err(syn::Error::new(kw.span(), "duplicate `before` in suite!"));
                    }
                    let body;
                    braced!(body in input);
                    before = Some(body.parse()?);
                }
                "before_each" => {
                    if before_each.is_some() {
                        return Err(syn::Error::new(
                            kw.span(),
                            "duplicate `before_each` in suite!",
                        ));
                    }
                    let body;
                    braced!(body in input);
                    before_each = Some(body.parse()?);
                }
                "after_each" => {
                    if after_each.is_some() {
                        return Err(syn::Error::new(
                            kw.span(),
                            "duplicate `after_each` in suite!",
                        ));
                    }
                    let body;
                    braced!(body in input);
                    after_each = Some(body.parse()?);
                }
                other => {
                    return Err(syn::Error::new(
                        kw.span(),
                        format!(
                            "unexpected `{other}` in suite! \
                             (expected `before`, `before_each`, or `after_each`)"
                        ),
                    ));
                }
            }
        }

        Ok(SuiteBlock {
            before,
            before_each,
            after_each,
        })
    }
}

/// Defines suite-level hooks. See [`spectacular::suite`](https://docs.rs/spectacular) for full docs.
#[proc_macro]
pub fn suite(input: TokenStream) -> TokenStream {
    let block = syn::parse_macro_input!(input as SuiteBlock);

    let before_body = block.before.unwrap_or_default();
    let before_each_body = block.before_each.unwrap_or_default();
    let after_each_body = block.after_each.unwrap_or_default();

    quote! {
        mod __spectacular_suite {
            use super::*;
            pub fn before() {
                static __ONCE: ::std::sync::Once = ::std::sync::Once::new();
                __ONCE.call_once(|| { #before_body });
            }
            pub fn before_each() { #before_each_body }
            pub fn after_each() { #after_each_body }
        }
    }
    .into()
}

/// RSpec-style test DSL. See [`spectacular::spec`](https://docs.rs/spectacular) for full docs.
#[proc_macro]
pub fn spec(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    match spec::expand(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

struct TestSuiteArgs {
    has_suite: bool,
    runtime: Option<Runtime>,
}

impl Parse for TestSuiteArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut has_suite = false;
        let mut runtime = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "suite" => has_suite = true,
                "tokio" => {
                    if runtime.is_some() {
                        return Err(syn::Error::new(ident.span(), "duplicate runtime specifier"));
                    }
                    runtime = Some(Runtime::Tokio);
                }
                "async_std" => {
                    if runtime.is_some() {
                        return Err(syn::Error::new(ident.span(), "duplicate runtime specifier"));
                    }
                    runtime = Some(Runtime::AsyncStd);
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unexpected `{other}` (expected `suite`, `tokio`, or `async_std`)"),
                    ));
                }
            }
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(TestSuiteArgs { has_suite, runtime })
    }
}

/// Attribute-style test suite. See [`spectacular::test_suite`](https://docs.rs/spectacular) for full docs.
#[proc_macro_attribute]
pub fn test_suite(attr_input: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr_input as TestSuiteArgs);
    let input = syn::parse_macro_input!(item as ItemMod);
    match attr::expand(input, args.has_suite, args.runtime) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Marker for once-per-group setup in [`test_suite`]. See [`spectacular::before`](https://docs.rs/spectacular).
#[proc_macro_attribute]
pub fn before(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marker for once-per-group teardown in [`test_suite`]. See [`spectacular::after`](https://docs.rs/spectacular).
#[proc_macro_attribute]
pub fn after(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marker for per-test setup in [`test_suite`]. See [`spectacular::before_each`](https://docs.rs/spectacular).
#[proc_macro_attribute]
pub fn before_each(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marker for per-test teardown in [`test_suite`]. See [`spectacular::after_each`](https://docs.rs/spectacular).
#[proc_macro_attribute]
pub fn after_each(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
