//! Attribute-style `#[test_suite]` — parsing and code generation.

use quote::quote;
use syn::{Ident, ItemFn, ItemMod};

use crate::wrap_test_body;

pub(crate) fn expand(input: ItemMod, has_suite: bool) -> syn::Result<proc_macro2::TokenStream> {
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
