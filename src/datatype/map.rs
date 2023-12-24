use crate::DataType;

#[derive(Debug, Clone, PartialEq)]

pub struct Map {
    // TODO: Box these fields together as an internal optimization.
    // The type of the map keys.
    pub(crate) key_ty: Box<DataType>,
    // The type of the map values.
    pub(crate) value_ty: Box<DataType>,
    // Are each elements unique? Eg. `HashSet` or `BTreeSet`
    pub(crate) unique: bool,
}

impl Map {
    // TODO: `inline` vs `reference` is a thing people downstream need to think about.
    // TODO: Should this need `generics`
    // pub fn new<K: Type, V: Type>(type_map: &mut TypeMap, generics: &[DataType]) -> Self {
    //     Self {
    //         key_ty: Box::new(K::inline(type_map, generics)),
    //         value_ty: Box::new(V::inline(type_map, generics)),
    //         unique: false,
    //     }
    // }

    pub fn key_ty(&self) -> &DataType {
        &self.key_ty
    }

    pub fn value_ty(&self) -> &DataType {
        &self.value_ty
    }

    pub fn unique(&self) -> bool {
        self.unique
    }
}
