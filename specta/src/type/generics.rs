//! We define a struct with implements `Type` for all generics we need in `impls.rs` and `legacy_impls.rs`
//! These allow us to transform the properly support generics.

use crate::{
    Type, Types,
    datatype::{self, DataType},
};

macro_rules! impl_generic {
    ($($ident:ident),* $(,)?) => {
        $(
            #[allow(dead_code)]
            pub(crate) struct $ident;

            impl Type for $ident {
                fn definition(_: &mut Types) -> DataType {
                    datatype::GenericReference::new::<Self>().into()
                }
            }
        )*
    };
}

impl_generic!(T, K, V, E, L, R);
