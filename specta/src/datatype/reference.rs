//! Helpers for generating [Type::reference] implementations.

use crate::{Generics, NamedType, SpectaID, Type, TypeCollection};

use super::{DataType, DataTypeReference};

/// A reference datatype.
///
// This type exists to force the user to use [reference::inline] or [reference::reference] which provides some extra safety.
#[non_exhaustive]
pub struct Reference {
    // TODO: Seal these fields
    pub sid: SpectaID,
    pub inner: DataType,
}

// TODO: I think inline in userspace is not gonna work well but we can try it.
// pub fn inlined<T: Type + ?Sized>(
//     type_map: &mut TypeCollection,
//     generics: &[DataType],
// ) -> DataType {
//     match T::reference(type_map, generics) {
//         Some(reference) => {
//             // TODO: This acts as an inlined value but will cause problems with generics.
//             let ty = type_map.get(reference.sid).unwrap(); // TODO: Error handling. This should be impossible.
//             ty.inner.clone()
//         },
//         None => T::inline(type_map, Generics::Provided(generics)),
//     }
// }

pub fn reference_or_inline<T: Type + ?Sized>(
    type_map: &mut TypeCollection,
    generics: &[DataType],
) -> DataType {
    match T::reference(type_map, generics) {
        Some(reference) => {
            // TODO: Get from type map instead

            // we wanna be able to go from a reference back into a `DataType::Reference`
            // let todo = DataType::Reference(DataTypeReference {
            //     name: todo!(),
            //     sid: todo!(),
            //     generics: Default::default(), // TODO: Fix this
            // });

            // TODO: This results in it being inlined which is not what we want.
            // let ty = type_map.get(reference.sid).unwrap(); // TODO: Error handling. This should be impossible.
            // ty.inner.clone()

            reference.inner
        },
        None => T::inline(type_map, Generics::Provided(generics)),
    }
}

pub fn inline<T: Type + ?Sized>(type_map: &mut TypeCollection, generics: &[DataType]) -> Reference {
    Reference {
        sid: todo!("sid a"),
        inner: T::inline(type_map, Generics::Provided(generics)),
    }
}

pub fn reference<T: NamedType>(
    type_map: &mut TypeCollection,
    // reference: DataTypeReference,
) -> Reference {
    let sid = T::sid();

    if type_map.map.get(&sid).is_none() {
        type_map.map.entry(sid).or_insert(None);
        let dt = T::definition_named_data_type(type_map);
        type_map.map.insert(sid, Some(dt));
    }

    Reference {
        sid: T::sid(),
        inner: DataType::Reference(DataTypeReference {
            // TODO: Make this stuff work
            // name: "".into(),
            sid,
            generics: Default::default()
        }),
    }
}

/// Construct a reference from a custom [DataType].
///
/// This function is advanced and should only be used if you know what you're doing.
pub fn custom(inner: DataType) -> Reference {
    Reference { sid: todo!("sid c"), inner }
}
