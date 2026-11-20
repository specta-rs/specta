//! Accepts `#[serde(...)]` attributes without pulling `serde_derive` into the
//! benchmark fixture, so the measured expansion is Specta's alone.

use proc_macro::TokenStream;

#[proc_macro_derive(SerdeShim, attributes(serde))]
pub fn serde_shim(_: TokenStream) -> TokenStream {
    TokenStream::new()
}
