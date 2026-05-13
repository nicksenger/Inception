use proc_macro::TokenStream;

pub use inception_derive_gen::Identifier;

pub struct State;

impl State {
    pub fn gen(input: TokenStream) -> TokenStream {
        inception_derive_gen::State::gen(input.into()).into()
    }
}
