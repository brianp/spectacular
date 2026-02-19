use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, ItemMod, LitStr, Token, braced};

fn slugify(s: &str) -> String {
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

fn wrap_test_body(
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

// ---- suite! macro ----

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

// ---- #[test_suite] attribute ----

#[proc_macro_attribute]
pub fn test_suite(attr: TokenStream, item: TokenStream) -> TokenStream {
    let has_suite = if attr.is_empty() {
        false
    } else {
        let ident = syn::parse_macro_input!(attr as Ident);
        if ident != "suite" {
            return syn::Error::new(ident.span(), "expected `suite`")
                .to_compile_error()
                .into();
        }
        true
    };
    let input = syn::parse_macro_input!(item as ItemMod);
    match expand_test_suite(input, has_suite) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_test_suite(input: ItemMod, has_suite: bool) -> syn::Result<proc_macro2::TokenStream> {
    let mod_name = &input.ident;
    let vis = &input.vis;

    let Some((_, items)) = &input.content else {
        return Err(syn::Error::new_spanned(
            &input,
            "#[test_suite] requires an inline module (not `mod foo;`)",
        ));
    };

    let mut before_fn: Option<&Ident> = None;
    let mut after_fn: Option<&Ident> = None;
    let mut before_each_fn: Option<&Ident> = None;
    let mut after_each_fn: Option<&Ident> = None;
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
                        before_fn = Some(&func.sig.ident);
                    } else if attr.path().is_ident("after") {
                        if after_fn.is_some() {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "only one #[after] per module",
                            ));
                        }
                        after_fn = Some(&func.sig.ident);
                    } else if attr.path().is_ident("before_each") {
                        if before_each_fn.is_some() {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "only one #[before_each] per module",
                            ));
                        }
                        before_each_fn = Some(&func.sig.ident);
                    } else if attr.path().is_ident("after_each") {
                        if after_each_fn.is_some() {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "only one #[after_each] per module",
                            ));
                        }
                        after_each_fn = Some(&func.sig.ident);
                    } else if attr.path().is_ident("test_case") {
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

    let has_before = before_fn.is_some();
    let has_after = after_fn.is_some();
    let has_after_each = after_each_fn.is_some();
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
            let other_attrs: Vec<_> = func
                .attrs
                .iter()
                .filter(|a| !a.path().is_ident("test_case"))
                .collect();

            let mut pre = proc_macro2::TokenStream::new();
            let mut post = proc_macro2::TokenStream::new();

            if has_suite {
                pre.extend(quote! { super::__spectacular_suite::before(); });
            }
            if let Some(name) = before_fn {
                pre.extend(quote! { __SPEC_BEFORE.call_once(#name); });
            }
            if has_suite {
                pre.extend(quote! { super::__spectacular_suite::before_each(); });
            }
            if let Some(name) = before_each_fn {
                pre.extend(quote! { #name(); });
            }

            if let Some(name) = after_each_fn {
                post.extend(quote! { #name(); });
            }
            if has_suite {
                post.extend(quote! { super::__spectacular_suite::after_each(); });
            }
            if let Some(name) = after_fn {
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
            let inner = wrap_test_body(pre, quote! { #body }, post, needs_catch);

            quote! {
                #(#other_attrs)*
                #[test]
                #fn_vis fn #fn_name() {
                    #inner
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

// ---- marker attributes ----

#[proc_macro_attribute]
pub fn test_case(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn before(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn after(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn before_each(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn after_each(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

// ---- spec! macro ----

enum SpecItem {
    Suite,
    Before(proc_macro2::TokenStream),
    After(proc_macro2::TokenStream),
    BeforeEach(proc_macro2::TokenStream),
    AfterEach(proc_macro2::TokenStream),
    It(String, proc_macro2::TokenStream),
    Other(proc_macro2::TokenStream),
}

struct SpecModule {
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
                    "it" => {
                        let _: Ident = content.parse()?;
                        let desc: LitStr = content.parse()?;
                        let body;
                        braced!(body in content);
                        items.push(SpecItem::It(desc.value(), body.parse()?));
                        continue;
                    }
                    "before_each" => {
                        let _: Ident = content.parse()?;
                        let body;
                        braced!(body in content);
                        items.push(SpecItem::BeforeEach(body.parse()?));
                        continue;
                    }
                    "after_each" => {
                        let _: Ident = content.parse()?;
                        let body;
                        braced!(body in content);
                        items.push(SpecItem::AfterEach(body.parse()?));
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

#[proc_macro]
pub fn spec(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    match expand_spec(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_spec(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let parsed: SpecModule = syn::parse2(input)?;
    let mod_name = &parsed.ident;
    let vis = &parsed.vis;

    let has_suite = parsed
        .items
        .iter()
        .any(|item| matches!(item, SpecItem::Suite));

    let mut before_body: Option<proc_macro2::TokenStream> = None;
    let mut after_body: Option<proc_macro2::TokenStream> = None;
    let mut before_each_body: Option<proc_macro2::TokenStream> = None;
    let mut after_each_body: Option<proc_macro2::TokenStream> = None;
    let mut tests: Vec<(String, Ident, proc_macro2::TokenStream)> = Vec::new();
    let mut other_items: Vec<proc_macro2::TokenStream> = Vec::new();

    for item in parsed.items {
        match item {
            SpecItem::Suite => {}
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
            SpecItem::BeforeEach(body) => {
                if before_each_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `before_each` block per module",
                    ));
                }
                before_each_body = Some(body);
            }
            SpecItem::AfterEach(body) => {
                if after_each_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `after_each` block per module",
                    ));
                }
                after_each_body = Some(body);
            }
            SpecItem::It(desc, body) => {
                let fn_name = format_ident!("{}", slugify(&desc));
                tests.push((desc, fn_name, body));
            }
            SpecItem::Other(tokens) => {
                other_items.push(tokens);
            }
        }
    }

    let has_before = before_body.is_some();
    let has_after = after_body.is_some();
    let has_before_each = before_each_body.is_some();
    let has_after_each = after_each_body.is_some();
    let test_count = tests.len();

    let before_fn = before_body.map(|body| quote! { fn __spec_before() { #body } });
    let after_fn = after_body.map(|body| quote! { fn __spec_after() { #body } });
    let before_each_fn = before_each_body.map(|body| quote! { fn __spec_before_each() { #body } });
    let after_each_fn = after_each_body.map(|body| quote! { fn __spec_after_each() { #body } });

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
        .map(|(_desc, fn_name, body)| {
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
                pre.extend(quote! { __spec_before_each(); });
            }

            if has_after_each {
                post.extend(quote! { __spec_after_each(); });
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
            let inner = wrap_test_body(pre, body.clone(), post, needs_catch);

            quote! {
                #[test]
                fn #fn_name() {
                    #inner
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
