use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
    Expr, ExprArray, Meta,
};

struct Attributes {
    properties: Vec<syn::Path>,
}

impl Parse for Attributes {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let metas = Punctuated::<Meta, Comma>::parse_terminated(input)?;
        let mut properties = None;

        for meta in metas {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("properties") => {
                    if properties.is_some() {
                        return Err(syn::Error::new_spanned(
                            nv.path,
                            "Duplicate `properties` setting.",
                        ));
                    }

                    let Expr::Array(ExprArray { elems, .. }) = nv.value else {
                        return Err(syn::Error::new_spanned(
                            nv,
                            "Expected `properties` to be an array expression.",
                        ));
                    };

                    let mut parsed = Vec::new();
                    for elem in elems {
                        let Expr::Path(path_expr) = elem else {
                            return Err(syn::Error::new_spanned(
                                elem,
                                "Expected each `properties` item to be a path.",
                            ));
                        };
                        parsed.push(path_expr.path);
                    }

                    properties = Some(parsed);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "Unknown `inception_derive` setting.",
                    ));
                }
            }
        }

        Ok(Self {
            properties: properties.unwrap_or_default(),
        })
    }
}

pub struct State {}
impl State {
    pub fn gen(attr: TokenStream, item: TokenStream) -> TokenStream {
        let input = parse_macro_input!(item as syn::Item);
        let Attributes { properties } = match syn::parse(attr) {
            Ok(attrs) => attrs,
            Err(e) => return e.into_compile_error().into(),
        };

        match input {
            syn::Item::Struct(_) | syn::Item::Enum(_) => {
                let props = if properties.is_empty() {
                    quote! {}
                } else {
                    quote! {
                        #[inception(properties = [#(#properties),*])]
                    }
                };

                quote! {
                    #[derive(inception::Inception)]
                    #props
                    #input
                }
                .into()
            }
            item => syn::Error::new_spanned(
                item,
                "This macro can only be applied to struct or enum definitions.",
            )
            .to_compile_error()
            .into(),
        }
    }
}
