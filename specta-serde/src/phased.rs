use std::marker::PhantomData;

use specta::{
    Type, TypeCollection,
    datatype::{DataType, Tuple},
};

pub struct Phased<Serialize, Deserialize> {
    phantom: PhantomData<(Serialize, Deserialize)>,
}

pub trait Phased2 {
    type Serialize;
    type Deserialize;
}
impl<Serialize, Deserialize> Phased2 for Phased<Serialize, Deserialize> {
    type Serialize = Serialize;
    type Deserialize = Deserialize;
}

impl<Serialize: Type, Deserialize: Type> Type for Phased<Serialize, Deserialize> {
    fn definition(types: &mut TypeCollection) -> DataType {
        let payload = DataType::Tuple(Tuple::new(vec![
            Serialize::definition(types),
            Deserialize::definition(types),
        ]));

        let ndt = NamedDataTypeBuilder::new("Phased", vec![], payload)
            .module_path("specta_serde")
            .inline()
            .build(types);

        DataType::Reference(ndt.reference(vec![]))
    }
}
