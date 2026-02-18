use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, ItemMod, LitStr, Token, braced};

#[proc_macro_attribute]
pub fn test_suite(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as ItemMod);
    match expand_test_suite(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_test_suite(input: ItemMod) -> syn::Result<proc_macro2::TokenStream> {
    let mod_name = &input.ident;
    let mod_name_str = mod_name.to_string();

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
                                "only one #[before] per #[test_suite] module",
                            ));
                        }
                        before_fn = Some(&func.sig.ident);
                    } else if attr.path().is_ident("after") {
                        if after_fn.is_some() {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "only one #[after] per #[test_suite] module",
                            ));
                        }
                        after_fn = Some(&func.sig.ident);
                    } else if attr.path().is_ident("before_each") {
                        if before_each_fn.is_some() {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "only one #[before_each] per #[test_suite] module",
                            ));
                        }
                        before_each_fn = Some(&func.sig.ident);
                    } else if attr.path().is_ident("after_each") {
                        if after_each_fn.is_some() {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "only one #[after_each] per #[test_suite] module",
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
            _ => {
                other_items.push(item);
            }
        }
    }

    let cleaned_items: Vec<proc_macro2::TokenStream> = other_items
        .iter()
        .map(|item| {
            if let syn::Item::Fn(func) = item {
                let mut clean_func = func.clone();
                clean_func.attrs.retain(|attr| {
                    !attr.path().is_ident("before")
                        && !attr.path().is_ident("after")
                        && !attr.path().is_ident("before_each")
                        && !attr.path().is_ident("after_each")
                });
                quote! { #clean_func }
            } else {
                quote! { #item }
            }
        })
        .collect();

    let test_fn_defs: Vec<proc_macro2::TokenStream> = test_fns
        .iter()
        .map(|func| {
            let mut clean = (*func).clone();
            clean
                .attrs
                .retain(|attr| !attr.path().is_ident("test_case"));
            quote! { #clean }
        })
        .collect();

    let test_submissions: Vec<proc_macro2::TokenStream> = test_fns
        .iter()
        .map(|func| {
            let fn_name = &func.sig.ident;
            let fn_name_str = fn_name.to_string();
            quote! {
                ::spectacular::inventory::submit! {
                    ::spectacular::types::TestCase {
                        name: #fn_name_str,
                        module: #mod_name_str,
                        test_fn: #mod_name::#fn_name,
                        file: file!(),
                        line: line!(),
                    }
                }
            }
        })
        .collect();

    let before_expr = option_fn_expr(before_fn, mod_name);
    let after_expr = option_fn_expr(after_fn, mod_name);
    let before_each_expr = option_fn_expr(before_each_fn, mod_name);
    let after_each_expr = option_fn_expr(after_each_fn, mod_name);

    let vis = &input.vis;

    Ok(quote! {
        #vis mod #mod_name {
            #(#cleaned_items)*
            #(#test_fn_defs)*
        }

        ::spectacular::inventory::submit! {
            ::spectacular::types::TestGroup {
                name: #mod_name_str,
                before: #before_expr,
                after: #after_expr,
                before_each: #before_each_expr,
                after_each: #after_each_expr,
            }
        }

        #(#test_submissions)*
    })
}

fn option_fn_expr(ident: Option<&Ident>, mod_name: &Ident) -> proc_macro2::TokenStream {
    match ident {
        Some(name) => quote! { Some(#mod_name::#name) },
        None => quote! { None },
    }
}

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

#[proc_macro]
pub fn spec(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    match expand_spec(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

enum SpecItem {
    Before(proc_macro2::TokenStream),
    After(proc_macro2::TokenStream),
    BeforeEach(proc_macro2::TokenStream),
    AfterEach(proc_macro2::TokenStream),
    It(String, proc_macro2::TokenStream),
    Other(proc_macro2::TokenStream),
}

fn expand_spec(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let parsed: SpecModule = syn::parse2(input)?;
    let mod_name = &parsed.ident;
    let mod_name_str = mod_name.to_string();
    let vis = &parsed.vis;

    let mut before_body: Option<proc_macro2::TokenStream> = None;
    let mut after_body: Option<proc_macro2::TokenStream> = None;
    let mut before_each_body: Option<proc_macro2::TokenStream> = None;
    let mut after_each_body: Option<proc_macro2::TokenStream> = None;
    let mut tests: Vec<(String, Ident, proc_macro2::TokenStream)> = Vec::new();
    let mut other_items: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut test_counter: usize = 0;

    for item in parsed.items {
        match item {
            SpecItem::Before(body) => {
                if before_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `before` block per #[spec] module",
                    ));
                }
                before_body = Some(body);
            }
            SpecItem::After(body) => {
                if after_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `after` block per #[spec] module",
                    ));
                }
                after_body = Some(body);
            }
            SpecItem::BeforeEach(body) => {
                if before_each_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `before_each` block per #[spec] module",
                    ));
                }
                before_each_body = Some(body);
            }
            SpecItem::AfterEach(body) => {
                if after_each_body.is_some() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "only one `after_each` block per #[spec] module",
                    ));
                }
                after_each_body = Some(body);
            }
            SpecItem::It(desc, body) => {
                let fn_name = format_ident!("__spec_test_{}", test_counter);
                tests.push((desc, fn_name, body));
                test_counter += 1;
            }
            SpecItem::Other(tokens) => {
                other_items.push(tokens);
            }
        }
    }

    let before_fn = before_body.as_ref().map(|body| {
        quote! { pub fn __spec_before() { #body } }
    });
    let after_fn = after_body.as_ref().map(|body| {
        quote! { pub fn __spec_after() { #body } }
    });
    let before_each_fn = before_each_body.as_ref().map(|body| {
        quote! { pub fn __spec_before_each() { #body } }
    });
    let after_each_fn = after_each_body.as_ref().map(|body| {
        quote! { pub fn __spec_after_each() { #body } }
    });

    let before_expr = if before_body.is_some() {
        quote! { Some(#mod_name::__spec_before) }
    } else {
        quote! { None }
    };
    let after_expr = if after_body.is_some() {
        quote! { Some(#mod_name::__spec_after) }
    } else {
        quote! { None }
    };
    let before_each_expr = if before_each_body.is_some() {
        quote! { Some(#mod_name::__spec_before_each) }
    } else {
        quote! { None }
    };
    let after_each_expr = if after_each_body.is_some() {
        quote! { Some(#mod_name::__spec_after_each) }
    } else {
        quote! { None }
    };

    let test_fn_defs: Vec<proc_macro2::TokenStream> = tests
        .iter()
        .map(|(_desc, fn_name, body)| {
            quote! { pub fn #fn_name() { #body } }
        })
        .collect();

    let test_submissions: Vec<proc_macro2::TokenStream> = tests
        .iter()
        .map(|(desc, fn_name, _body)| {
            quote! {
                ::spectacular::inventory::submit! {
                    ::spectacular::types::TestCase {
                        name: #desc,
                        module: #mod_name_str,
                        test_fn: #mod_name::#fn_name,
                        file: file!(),
                        line: line!(),
                    }
                }
            }
        })
        .collect();

    Ok(quote! {
        #vis mod #mod_name {
            #(#other_items)*
            #before_fn
            #after_fn
            #before_each_fn
            #after_each_fn
            #(#test_fn_defs)*
        }

        ::spectacular::inventory::submit! {
            ::spectacular::types::TestGroup {
                name: #mod_name_str,
                before: #before_expr,
                after: #after_expr,
                before_each: #before_each_expr,
                after_each: #after_each_expr,
            }
        }

        #(#test_submissions)*
    })
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
                let ident_lookahead = content.fork();
                let kw: Ident = ident_lookahead.parse()?;

                match kw.to_string().as_str() {
                    "it" => {
                        let _: Ident = content.parse()?;
                        let desc: LitStr = content.parse()?;
                        let body;
                        braced!(body in content);
                        let body_tokens: proc_macro2::TokenStream = body.parse()?;
                        items.push(SpecItem::It(desc.value(), body_tokens));
                        continue;
                    }
                    "before_each" => {
                        let _: Ident = content.parse()?;
                        let body;
                        braced!(body in content);
                        let body_tokens: proc_macro2::TokenStream = body.parse()?;
                        items.push(SpecItem::BeforeEach(body_tokens));
                        continue;
                    }
                    "after_each" => {
                        let _: Ident = content.parse()?;
                        let body;
                        braced!(body in content);
                        let body_tokens: proc_macro2::TokenStream = body.parse()?;
                        items.push(SpecItem::AfterEach(body_tokens));
                        continue;
                    }
                    "before" => {
                        let _: Ident = content.parse()?;
                        if content.peek(syn::token::Brace) {
                            let body;
                            braced!(body in content);
                            let body_tokens: proc_macro2::TokenStream = body.parse()?;
                            items.push(SpecItem::Before(body_tokens));
                            continue;
                        } else {
                            return Err(
                                content.error("expected `{` after `before` in #[spec] module")
                            );
                        }
                    }
                    "after" => {
                        let _: Ident = content.parse()?;
                        if content.peek(syn::token::Brace) {
                            let body;
                            braced!(body in content);
                            let body_tokens: proc_macro2::TokenStream = body.parse()?;
                            items.push(SpecItem::After(body_tokens));
                            continue;
                        } else {
                            return Err(
                                content.error("expected `{` after `after` in #[spec] module")
                            );
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
