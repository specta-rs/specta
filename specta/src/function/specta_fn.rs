use std::borrow::Cow;

use crate::{datatype::DeprecatedType, datatype::Function, TypeCollection};

use super::{FunctionArg, FunctionResult};

/// Implemented by functions that can be annotated with [`specta`](crate::specta).
///
/// This trait is sealed as it won't need to be used externally.
pub trait SpectaFn<TMarker> {
    /// Gets the type of a function as a [`Function`](crate::datatype::Function).
    fn to_datatype(
        asyncness: bool,
        name: Cow<'static, str>,
        types: &mut TypeCollection,
        fields: &[Cow<'static, str>],
        docs: Cow<'static, str>,
        deprecated: Option<DeprecatedType>,
        no_return_type: bool,
    ) -> Function;
}

impl<TResultMarker, TResult: FunctionResult<TResultMarker>> SpectaFn<TResultMarker>
    for fn() -> TResult
{
    fn to_datatype(
        asyncness: bool,
        name: Cow<'static, str>,
        types: &mut TypeCollection,
        _fields: &[Cow<'static, str>],
        docs: Cow<'static, str>,
        deprecated: Option<DeprecatedType>,
        no_return_type: bool,
    ) -> Function {
        Function {
            asyncness,
            name,
            args: vec![],
            result: (!no_return_type).then(|| TResult::to_datatype(types)),
            docs,
            deprecated,
        }
    }
}

macro_rules! impl_typed_command {
    ( impl $($i:ident),* ) => {
       paste::paste! {
            impl<
                TResultMarker,
                TResult: FunctionResult<TResultMarker>,
                $($i: FunctionArg),*
            > SpectaFn<TResultMarker> for fn($($i),*) -> TResult {
                fn to_datatype(
                    asyncness: bool,
                    name: Cow<'static, str>,
                    types: &mut TypeCollection,
                    fields: &[Cow<'static, str>],
                    docs: Cow<'static, str>,
                    deprecated: Option<DeprecatedType>,
                    no_return_type: bool,
                ) -> Function {
                    let mut fields = fields.into_iter();

                    Function {
                        asyncness,
                        name,
                        docs,
                        deprecated,
                        args: [$(
                            fields
                                .next()
                                .map_or_else(
                                    || None,
                                    |field| $i::to_datatype(types).map(|ty| (field.clone(), ty))
                                )
                        ),*,]
                            .into_iter()
                            .filter_map(|v| v)
                            .collect::<Vec<_>>(),
                        result: (!no_return_type).then(|| TResult::to_datatype(types)),
                    }
                }
            }
        }
    };
    ( $i2:ident $(, $i:ident)* ) => {
        impl_typed_command!(impl $i2 $(, $i)* );
        impl_typed_command!($($i),*);
    };
    () => {};
}

impl_typed_command!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
