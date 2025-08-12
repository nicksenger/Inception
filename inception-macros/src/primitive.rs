use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl, Path};

#[derive(deluxe::ParseMetaItem, deluxe::ExtractAttributes)]
#[deluxe(attributes(primitive))]
struct Attributes {
    property: Path,
}

pub struct State {}
impl State {
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
                let (impl_generics, ty_generics, where_clause) = x.generics.split_for_impl();

                quote! {
                    #input
                    const _: () = {
                        impl #impl_generics ::inception::IsPrimitive<#property> for #self_ty #ty_generics #where_clause {
                            type Is = <#property as ::inception::Compat<Self>>::Out;
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
