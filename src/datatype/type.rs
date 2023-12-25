use std::{any::TypeId, panic::Location};

use crate::{internal::type_id::non_static_type_id, DataType, Type};

#[derive(Debug)]
pub struct TypeImpl {
    name: &'static str,
    tid: TypeId,
    file: &'static str,
    line: u32,
    column: u32,
    // TODO: Not `pub`
    pub ty: DataType,
}

// TODO: Debug impl + ordering

impl TypeImpl {
    // TODO: Do we make this private for the macro only??? Probs
    // TODO: Anyone could call this on any type but the `caller` information will be inconsistent.
    #[track_caller]
    pub fn new<T: Type>(ty: DataType) -> Self {
        let caller = Location::caller();
        Self {
            name: std::any::type_name::<T>(),
            tid: non_static_type_id::<T>(),
            file: caller.file(),
            line: caller.line(),
            column: caller.column(),
            ty,
        }
    }

    // TODO: Field accessors

    // TODO: Iterator for module path derived from `name` field

    pub fn is<T: Type>() -> bool {
        todo!();
    }
}
