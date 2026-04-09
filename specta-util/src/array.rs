use std::marker::PhantomData;

use specta::{
    datatype::{DataType, List},
    Type, Types,
};

/// Declares a fixed-length array type for Specta exporters.
///
/// This is primarily useful with `#[specta(type = ...)]` when you want an array
/// field to keep its length information in exported schemas.
///
/// A plain Rust array like `[u8; 2]` may be exported as `number[]` if Specta
/// deems it's unable to safely export it as `[number, number]`. This limitation
/// is due to the fact that Specta can't track the inference of const generics and
/// hence can't fully support them. Using `FixedArray<N, T>` will always encode the
/// length so it can be used to force override Specta's conservative behaviour when you know what your doing.
///
/// ```ignore
/// use specta::Type;
///
/// /// #[derive(Type)]
/// struct DemoA {
///     a: [u8; 2],     // becomes `[number, number]`
///
///     #[specta(type = specta_util::FixedArray<2, u8>)]
///     d: [u8; 2],     // becomes `[number, number]`
/// }
///
/// #[derive(Type)]
/// struct DemoB<const N: usize = 1> {
///     // These are generalised by Specta as we can't know if a specific type is using `N` or a constant, and we don't know what `N` is.
///     // If you `#[specta(inline)]` or `#[serde(flatten)]` the `[number, number]` will be restored as we are able to track it properly.
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
        // Refer to the type documentation for the safety around this.
        l.length = Some(N);
        DataType::List(l)
    }
}
