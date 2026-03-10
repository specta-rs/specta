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
            fn definition(types: &mut specta::TypeCollection) -> specta::datatype::DataType {
                let ty = <$ty as specta::Type>::definition(types);
                let brand: &'static str = branded!(@brand $ident $( $ts_name )?);

                specta::datatype::DataType::Reference(
                    specta::datatype::Reference::opaque(
                        $crate::Branded::new(std::borrow::Cow::Borrowed(brand), ty)
                    )
                )
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
            fn definition(types: &mut specta::TypeCollection) -> specta::datatype::DataType {
                let ty = <$ty as specta::Type>::definition(types);
                let brand: &'static str = branded!(@brand $ident $( $ts_name )?);

                specta::datatype::DataType::Reference(
                    specta::datatype::Reference::opaque(
                        $crate::Branded::new(std::borrow::Cow::Borrowed(brand), ty)
                    )
                )
            }
        }
    };

    // Internal
     (@brand $ident:ident $ts_name:literal) => {
         $ts_name
     };
     (@brand $ident:ident) => {
         stringify!($ident)
     };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// Runtime payload for a TypeScript branded type.
pub struct Branded {
    brand: Cow<'static, str>,
    ty: DataType,
}

impl Branded {
    /// Construct a branded type from a brand label and inner type.
    pub fn new(brand: impl Into<Cow<'static, str>>, ty: DataType) -> Self {
        Self {
            brand: brand.into(),
            ty,
        }
    }

    /// Get the brand label.
    pub fn brand(&self) -> &Cow<'static, str> {
        &self.brand
    }

    /// Get the inner data type.
    pub fn ty(&self) -> &DataType {
        &self.ty
    }
}
