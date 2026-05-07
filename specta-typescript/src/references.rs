use std::{cell::RefCell, collections::HashSet};

use specta::datatype::NamedReference;

thread_local! {
    static REFERENCED_TYPES: RefCell<Option<Vec<HashSet<NamedReference>>>> = const { RefCell::new(None) };
    static MODULE_PATH_CONTEXT: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn with_module_path<R>(module_path: &str, func: impl FnOnce() -> R) -> R {
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            MODULE_PATH_CONTEXT.with_borrow_mut(|ctx| {
                ctx.pop();
            });
        }
    }

    MODULE_PATH_CONTEXT.with_borrow_mut(|ctx| {
        ctx.push(module_path.to_string());
    });

    let _guard = Guard;
    func()
}

pub(crate) fn current_module_path() -> Option<String> {
    MODULE_PATH_CONTEXT.with_borrow(|ctx| ctx.last().cloned())
}

/// This function collects all Typescript references which are created within the given closure.
///
/// This can be used for determining the imports required in a particular file.
pub fn collect_references<R>(func: impl FnOnce() -> R) -> (R, HashSet<NamedReference>) {
    struct Guard {
        armed: bool,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            if !self.armed {
                return;
            }

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

    let mut guard = Guard { armed: true };
    let result = func();
    let referenced_types = REFERENCED_TYPES.with_borrow_mut(|types| {
        let contexts = types
            .as_mut()
            .expect("COLLECTED_TYPES is unset but it should be set");
        let referenced_types = contexts
            .pop()
            .expect("COLLECTED_TYPES is missing a valid collection context");
        if contexts.is_empty() {
            *types = None;
        }
        referenced_types
    });
    guard.armed = false;

    (result, referenced_types)
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
