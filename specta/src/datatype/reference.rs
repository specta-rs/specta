//! Helpers for generating [Type::reference] implementations.

use crate::{Generics, NamedType, Type, TypeCollection};

use super::{DataType, DataTypeReference};

/// A reference datatype.
///
// This type exists to force the user to use [reference::inline] or [reference::reference] which provides some extra safety.
#[non_exhaustive]
pub struct Reference {
    pub inner: DataType,
}

pub fn inline<T: Type + ?Sized>(type_map: &mut TypeCollection, generics: &[DataType]) -> Reference {
    Reference {
        inner: T::inline(type_map, Generics::Provided(generics)),
    }
}

pub fn reference<T: NamedType>(
    type_map: &mut TypeCollection,
    reference: DataTypeReference,
) -> Reference {
    let sid = T::sid();

    if type_map.map.get(&sid).is_none() {
        type_map.map.entry(sid).or_insert(None);
        let dt = T::definition_named_data_type(type_map);
        type_map.map.insert(sid, Some(dt));
    }

    Reference {
        inner: DataType::Reference(reference),
    }
}

/// Construct a reference from a custom [DataType].
///
/// This function is advanced and should only be used if you know what you're doing.
pub fn custom(inner: DataType) -> Reference {
    Reference { inner }
}
