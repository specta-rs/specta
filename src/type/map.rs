use std::collections::BTreeMap;

use crate::{NamedDataType, SpectaID};

/// A map used to store the types "discovered" while exporting a type.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TypeMap {
    // `None` indicates that the entry is a placeholder. It was reference and we are currently working out it's definition.
    pub(crate) map: BTreeMap<SpectaID, Option<NamedDataType>>,
}

impl TypeMap {
    pub fn get(&self, sid: SpectaID) -> Option<&NamedDataType> {
        self.map.get(&sid).as_ref().and_then(|v| match v {
            Some(ndt) => Some(ndt),
            // If this method is used during type construction this case could be hit when it's actually valid
            // but all references are managed within `specta` so we can bypass this method and use `map` directly because we have `pub(crate)` access.
            None => {
                #[cfg(debug_assertions)]
                unreachable!("specta: `TypeMap::get` found a type placeholder!");
                #[cfg(not(debug_assertions))]
                None
            }
        })
    }

    pub fn insert(&mut self, sid: SpectaID, dt: NamedDataType) {
        self.map.insert(sid, Some(dt));
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    // TODO: It would be nice if this would a proper `Iterator` or `IntoIterator` implementation!
    pub fn iter(&self) -> impl Iterator<Item = (SpectaID, &NamedDataType)> {
        self.map.iter().filter_map(|(sid, ndt)| match ndt {
            Some(ndt) => Some((*sid, ndt)),
            None => {
                #[cfg(debug_assertions)]
                unreachable!("specta: `TypeMap::into_iter` found a type placeholder!");
                #[cfg(not(debug_assertions))]
                None
            }
        })
    }
}
