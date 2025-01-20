//! TODO: Remove all of these in future.

use specta::{datatype::inline_reference, internal::detect_duplicate_type_names, NamedType, Type, TypeCollection};
use specta_serde::is_valid_ty;

use crate::{primitives, ExportError, Typescript};


#[allow(missing_docs)]
pub type Result<T> = std::result::Result<T, ExportError>;

pub(crate) type Output = Result<String>;

/// Convert a type which implements [`Type`] to a TypeScript string with an export.
///
/// Eg. `export type Foo = { demo: string; };`
pub fn export_ref<T: NamedType>(_: &T, ts: &Typescript) -> Output {
    export::<T>(ts)
}

/// Convert a type which implements [`Type`] to a TypeScript string with an export.
///
/// Eg. `export type Foo = { demo: string; };`
pub fn export<T: NamedType>(ts: &Typescript) -> Output {
    let mut types = TypeCollection::default();
    T::definition(&mut types);
    let dt = types.get(T::ID).unwrap();
    is_valid_ty(&dt.inner, &types)?;
    if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&types).into_iter().next() {
        return Err(ExportError::DuplicateTypeName(ty_name, l0, l1));
    }

    primitives::export(ts, &types, dt)
}

/// Convert a type which implements [`Type`] to a TypeScript string.
///
/// Eg. `{ demo: string; };`
pub fn inline_ref<T: Type>(_: &T, ts: &Typescript) -> Output {
    inline::<T>(ts)
}

/// Convert a type which implements [`Type`] to a TypeScript string.
///
/// Eg. `{ demo: string; };`
pub fn inline<T: Type>(ts: &Typescript) -> Output {
    let mut types = TypeCollection::default();
    let dt = inline_reference::<T>(&mut types);
    is_valid_ty(&dt, &types)?;
    if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&types).into_iter().next() {
        return Err(ExportError::DuplicateTypeName(ty_name, l0, l1));
    }

    primitives::inline(ts, &types, &dt)
}
