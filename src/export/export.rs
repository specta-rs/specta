use crate::ts::TsExportError;
use crate::*;
use once_cell::sync::Lazy;
use std::collections::BTreeSet;
use std::sync::{PoisonError, RwLock, RwLockReadGuard};

type InnerTy = (TypeMap, BTreeSet<ExportError>);

// Global type store for collecting custom types to export.
static TYPES: Lazy<RwLock<InnerTy>> = Lazy::new(Default::default);

/// A lock type for iterating over the internal type map.
///
/// Holding this type will prevent any new types from being registered until it is dropped.
///
pub struct TypesIter {
    index: usize,
    lock: RwLockReadGuard<'static, InnerTy>,
}

impl Iterator for TypesIter {
    type Item = (SpectaID, Option<NamedDataType>);

    fn next(&mut self) -> Option<Self::Item> {
        let (k, v) = self.lock.0.iter().nth(self.index)?;
        self.index += 1;
        // We have to clone, because we can't invent a lifetime
        Some((*k, v.clone()))
    }
}

/// Get the global type store for collecting custom types to export.
pub fn get_types() -> Result<TypesIter, TsExportError> {
    let types = TYPES.read().unwrap_or_else(PoisonError::into_inner);

    // TODO: Return all errors at once?
    if let Some(err) = types.1.iter().next() {
        return Err(err.clone().into());
    }

    Ok(TypesIter {
        index: 0,
        lock: types,
    })
}

// Called within ctor functions to register a type.
#[doc(hidden)]
pub fn register_ty<T: Type>() -> () {
    let (type_map, errors) = &mut *TYPES.write().unwrap_or_else(PoisonError::into_inner);

    if let Err(err) = T::reference(
        DefOpts {
            parent_inline: false,
            type_map,
        },
        &[],
    ) {
        errors.insert(err);
    }
}
