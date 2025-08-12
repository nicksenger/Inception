#![allow(unused_attributes)]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;

mod derive;
mod inception;
mod primitive;

#[proc_macro_derive(Inception, attributes(inception))]
pub fn derive(input: TokenStream) -> TokenStream {
    derive::State::gen(input)
}

#[proc_macro_attribute]
pub fn inception(attr: TokenStream, item: TokenStream) -> TokenStream {
    inception::State::gen(attr, item)
}

#[proc_macro_attribute]
pub fn primitive(attr: TokenStream, item: TokenStream) -> TokenStream {
    primitive::State::gen(attr, item)
}
