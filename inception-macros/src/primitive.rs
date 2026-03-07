use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;
use std::collections::HashSet;
use syn::{parse_macro_input, GenericParam, ItemImpl, Path};

#[derive(deluxe::ParseMetaItem, deluxe::ExtractAttributes)]
#[deluxe(attributes(primitive))]
struct Attributes {
    property: Path,
}

pub struct State {}
impl State {
    fn collect_ident_names(tokens: proc_macro2::TokenStream) -> HashSet<String> {
        fn walk(stream: proc_macro2::TokenStream, names: &mut HashSet<String>) {
            for tt in stream {
                match tt {
                    TokenTree::Ident(id) => {
                        names.insert(id.to_string());
                    }
                    TokenTree::Group(g) => walk(g.stream(), names),
                    TokenTree::Punct(_) | TokenTree::Literal(_) => {}
                }
            }
        }

        let mut names = HashSet::new();
        walk(tokens, &mut names);
        names
    }

    pub fn gen(attr: TokenStream, item: TokenStream) -> TokenStream {
        let input = parse_macro_input!(item as syn::Item);
        match input {
            syn::Item::Impl(ref x) => {
                let ItemImpl {
                    trait_: Some(_t),
                    self_ty,
                    ..
                } = x
                else {
                    return syn::Error::new_spanned(
                        x,
                        "This macro can only be applied to trait implementations.",
                    )
                    .to_compile_error()
                    .into();
                };
                let Ok(Attributes { property }) = deluxe::parse(attr) else {
                    return syn::Error::new_spanned(x, "Expected \"property = ...\"")
                        .into_compile_error()
                        .into();
                };
                let self_ty_tokens = quote! { #self_ty }.to_string();
                let retained_params = x
                    .generics
                    .params
                    .iter()
                    .filter(|p| match p {
                        GenericParam::Type(t) => self_ty_tokens.contains(&t.ident.to_string()),
                        GenericParam::Lifetime(l) => {
                            self_ty_tokens.contains(&l.lifetime.ident.to_string())
                        }
                        GenericParam::Const(c) => self_ty_tokens.contains(&c.ident.to_string()),
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let retained_param_names = retained_params
                    .iter()
                    .map(|p| match p {
                        GenericParam::Type(t) => t.ident.to_string(),
                        GenericParam::Lifetime(l) => l.lifetime.ident.to_string(),
                        GenericParam::Const(c) => c.ident.to_string(),
                    })
                    .collect::<HashSet<_>>();
                let dropped_param_names = x
                    .generics
                    .params
                    .iter()
                    .map(|p| match p {
                        GenericParam::Type(t) => t.ident.to_string(),
                        GenericParam::Lifetime(l) => l.lifetime.ident.to_string(),
                        GenericParam::Const(c) => c.ident.to_string(),
                    })
                    .filter(|name| !retained_param_names.contains(name))
                    .collect::<HashSet<_>>();
                let impl_generics = if retained_params.is_empty() {
                    quote! {}
                } else {
                    quote! { <#(#retained_params),*> }
                };
                let retained_where_predicates = x
                    .generics
                    .where_clause
                    .as_ref()
                    .map(|wc| {
                        wc.predicates
                            .iter()
                            .filter(|pred| {
                                let used_names = Self::collect_ident_names(quote! { #pred });
                                !used_names
                                    .iter()
                                    .any(|name| dropped_param_names.contains(name))
                            })
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let impl_where_clause = if retained_where_predicates.is_empty() {
                    quote! {}
                } else {
                    quote! { where #(#retained_where_predicates),* }
                };

                quote! {
                    #input
                    const _: () = {
                        impl #impl_generics ::inception::IsPrimitive<#property> for #self_ty #impl_where_clause {
                            type Is = ::inception::True;
                        }
                    };
                }
                .into()
            }
            item => syn::Error::new_spanned(
                item,
                "This macro can only be applied to trait implementations.",
            )
            .to_compile_error()
            .into(),
        }
    }
}
