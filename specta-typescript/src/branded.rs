use std::borrow::Cow;

use specta::datatype::DataType;

/// Create a branded tuple struct type that exports to TypeScript with a custom name.
///
/// This macro generates a single-field tuple struct and implements the `Type` trait
/// for it, allowing you to create "branded" types that maintain distinct identities
/// in TypeScript.
///
/// # Examples
///
/// Basic usage:
/// ```ignore
/// branded!(pub struct AccountId(String));
/// ```
///
/// With custom TypeScript name:
/// ```ignore
/// branded!(pub struct AccountId(String) as "accountId");
/// ```
///
/// With attributes:
/// ```ignore
/// branded!(#[derive(Serialize)] pub struct UserId(String));
/// ```
///
/// With generics:
/// ```ignore
/// branded!(pub struct Id<T>(T) as "id");
/// ```
///
/// # Requirements
///
/// This macro requires that the `specta` crate is in scope and available as a dependency.
///
/// # Notes
///
/// - The struct must be a tuple struct with exactly one field
/// - The `Type` implementation is currently a `todo!()` placeholder
/// - The `as "name"` syntax is optional; if omitted, the struct name is used
#[macro_export]
macro_rules! branded {
    // Pattern with generics and optional TypeScript name
    (
        $(#[$attr:meta])*
        $vis:vis struct $ident:ident<$($generic:ident),+ $(,)?> ( $ty:ty ) $(as $ts_name:literal)?
    ) => {
        $(#[$attr])*
        $vis struct $ident<$($generic),+>($ty);

        impl<$($generic: specta::Type),+> specta::Type for $ident<$($generic),+> {
            fn definition(_types: &mut specta::TypeCollection) -> specta::DataType {
                todo!("branded type implementation for {}", stringify!($ident))
            }
        }
    };

    // Pattern without generics
    (
        $(#[$attr:meta])*
        $vis:vis struct $ident:ident ( $ty:ty ) $(as $ts_name:literal)?
    ) => {
        $(#[$attr])*
        $vis struct $ident($ty);

        impl specta::Type for $ident {
            fn definition(_types: &mut specta::TypeCollection) -> specta::DataType {
                todo!("branded type implementation for {}", stringify!($ident))
            }
        }
    };
}

#[derive(Debug, Clone)] // TODO
pub struct Branded {
    brand: Cow<'static, str>,
    ty: DataType,
}

impl Branded {
    pub fn new(brand: Cow<'static, str>, ty: DataType) -> Self {
        Self { brand, ty }
    }

    pub fn brand(&self) -> &Cow<'static, str> {
        &self.brand
    }

    pub fn ty(&self) -> &DataType {
        &self.ty
    }
}
