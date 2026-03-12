use std::marker::PhantomData;

use specta::{
    Type, Types,
    datatype::{DataType, NamedDataType, Tuple},
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
pub struct Phased<Serialize, Deserialize> {
    phantom: PhantomData<(Serialize, Deserialize)>,
}

impl<Serialize: Type, Deserialize: Type> Type for Phased<Serialize, Deserialize> {
    fn definition(types: &mut Types) -> DataType {
        let ser = Serialize::definition(types);
        let der = Deserialize::definition(types);

        if ser == der {
            ser
        } else {
            let payload = DataType::Tuple(Tuple::new(vec![
                Serialize::definition(types),
                Deserialize::definition(types),
            ]));

            let mut ndt = NamedDataType::new_inline("Phased", vec![], payload);
            ndt.set_module_path("specta_serde".into());
            ndt.register(types);

            DataType::Reference(ndt.reference(vec![]))
        }
    }
}
