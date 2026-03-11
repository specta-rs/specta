use std::marker::PhantomData;

use specta::{
    datatype::{DataType, NamedDataType, Tuple},
    Type, TypeCollection,
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

        let mut ndt = NamedDataType::new_inline("Phased", vec![], payload);
        ndt.set_module_path("specta_serde".into());
        ndt.register(types);

        DataType::Reference(ndt.reference(vec![]))
    }
}
