use std::{cell::RefCell, collections::HashSet};

use specta::datatype::NamedReference;

thread_local! {
    static REFERENCED_TYPES: RefCell<Option<Vec<HashSet<NamedReference>>>> = const { RefCell::new(None) };
}

/// This function collects all Typescript references which are created within the given closure.
///
/// This can be used for determining the imports required in a particular file.
pub fn collect_references<R>(func: impl FnOnce() -> R) -> (R, HashSet<NamedReference>) {
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            REFERENCED_TYPES.with_borrow_mut(|types| {
                if let Some(v) = types {
                    // Last collection means we can drop all memory
                    if v.len() == 1 {
                        *types = None;
                    } else {
                        // Otherwise just remove the current collection.
                        v.pop();
                    }
                }
            })
        }
    }

    // If we have no collection, register one
    // If we already have one create a new context.
    REFERENCED_TYPES.with_borrow_mut(|v| {
        if let Some(v) = v {
            v.push(Default::default());
        } else {
            *v = Some(vec![Default::default()]);
        }
    });

    let guard = Guard;
    let result = func();
    // We only use the guard when unwinding
    std::mem::forget(guard);

    (
        result,
        REFERENCED_TYPES.with_borrow_mut(|types| {
            types
                .as_mut()
                .expect("COLLECTED_TYPES is unset but it should be set")
                .pop()
                .expect("COLLECTED_TYPES is missing a valid collection context")
        }),
    )
}

/// Used internally to track a named references.
pub(crate) fn track_nr(r: &NamedReference) {
    REFERENCED_TYPES.with_borrow_mut(|ctxs| {
        if let Some(ctxs) = ctxs {
            for ctx in ctxs {
                ctx.insert(r.clone());
            }
        }
    });
}
