use std::marker::PhantomData;

use specta::{
    Type, Types,
    datatype::{DataType, List},
};

/// Declares a fixed-length array type for Specta exporters.
///
/// This is primarily useful with `#[specta(type = ...)]` when you want an array
/// field to keep its length information in exported schemas.
///
/// A plain Rust array like `[u8; 2]` may be exported as `number[]` in generic
/// contexts, because the named Specta type cannot safely encode every possible
/// const-generic instantiation. `FixedArray<N, T>` lets you opt into a concrete
/// fixed-length representation for a specific field instead.
///
/// Exporters that understand fixed-length lists can use this metadata to emit a
/// tuple-like representation such as `[number, number]`.
///
/// ```ignore
/// use specta::Type;
///
/// #[derive(Type)]
/// struct Demo<const N: usize = 1> {
///     data: [u32; N], // becomes `number[]`
///     a: [u8; 2],     // becomes `number[]`
///
///     #[specta(type = specta_util::FixedArray<2, u8>)]
///     d: [u8; 2],     // becomes `[number, number]`
/// }
/// ```
pub struct FixedArray<const N: usize, T: Type>(PhantomData<[T; N]>);

impl<const N: usize, T: Type> Type for FixedArray<N, T> {
    fn definition(types: &mut Types) -> DataType {
        let mut l = List::new(T::definition(types));
        // TODO: Explain safety
        l.set_length(Some(N));
        DataType::List(l)
    }
}
