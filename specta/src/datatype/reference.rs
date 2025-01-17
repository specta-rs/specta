//! Helpers for generating [Type::reference] implementations.

use crate::{Generics, NamedType, SpectaID, Type, TypeCollection};

use super::{DataType, DataTypeReference, GenericType};

/// A reference datatype.
///
/// TODO: Explain how to construct this.
#[non_exhaustive]
pub struct Reference {
    pub(crate) sid: SpectaID,
}

impl Reference {
    /// TODO: Explain invariant.
    pub fn construct(sid: SpectaID) -> Self {
        Self { sid }
    }

    // pub fn new(sid: SpectaID) -> Self {
    //     //     if type_map.map.get(&sid).is_none() {
    //     //         type_map.map.entry(sid).or_insert(None);
    //     //         let dt = T::definition_named_data_type(type_map);
    //     //         type_map.map.insert(sid, Some(dt));
    //     //     }

    //     Self {
    //         sid,
    //     }
    // }

    pub fn sid(&self) -> SpectaID {
        self.sid
    }

    pub fn to_datatype(&self, generics: impl Into<Vec<(GenericType, DataType)>>) -> DataType {
        DataType::Reference(DataTypeReference {
            sid: self.sid,
            generics: generics.into(),
        })
    }
}

pub fn reference_or_inline<T: Type + ?Sized>(
    types: &mut TypeCollection,
    generics: &[DataType],
) -> DataType {
    match T::reference(types, generics) {
        Some(reference) => {
            // TODO: Fix generics
            reference.to_datatype(vec![])
        },
        None => T::inline(types, Generics::Provided(generics)),
    }
}

// TODO: Remove this?
pub fn reference<T: NamedType>(
    type_map: &mut TypeCollection,
) -> Reference {
    let sid = T::sid();

    if type_map.map.get(&sid).is_none() {
        type_map.map.entry(sid).or_insert(None);
        let dt = T::definition_named_data_type(type_map);
        type_map.map.insert(sid, Some(dt));
    }

    Reference {
        sid: T::sid(),
        // inner: DataType::Reference(DataTypeReference {
        //     sid,
        //     // TODO: Make this work
        //     generics: Default::default()
        // }),
    }
}
