use std::marker::PhantomData;

use specta::{
    Type, Types,
    datatype::{DataType, Reference},
};

/// Declares an explicit serialize/deserialize type pair for Specta output.
///
/// This is primarily used with `#[specta(type = ...)]` when serde attributes
/// cause the wire shape to differ by direction.
///
/// - `Serialize` is the type used for serialization output.
/// - `Deserialize` is the type accepted for deserialization input.
///
/// When both phases resolve to the same Specta datatype, this collapses to that
/// single type. When they differ, `apply_phases` can split the graph into
/// `*_Serialize` and `*_Deserialize` variants.
///
/// ```rust
/// # use specta::Type;
/// #[derive(Type)]
/// struct OneOrMany {
///     #[specta(type = specta_serde::Phased<Vec<String>, String>)]
///     value: Vec<String>,
/// }
/// ```
pub struct Phased<Serialize, Deserialize>(PhantomData<(Serialize, Deserialize)>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PhasedTy {
    pub(crate) serialize: DataType,
    pub(crate) deserialize: DataType,
}

/// Builds an explicit phased [`DataType`] from precomputed serialize and deserialize shapes.
pub fn phased(serialize: DataType, deserialize: DataType) -> DataType {
    if serialize == deserialize {
        serialize
    } else {
        DataType::Reference(Reference::opaque(PhasedTy {
            serialize,
            deserialize,
        }))
    }
}

impl<Serialize: Type, Deserialize: Type> Type for Phased<Serialize, Deserialize> {
    fn definition(types: &mut Types) -> DataType {
        let serialize = Serialize::definition(types);
        let deserialize = Deserialize::definition(types);

        phased(serialize, deserialize)
    }
}
