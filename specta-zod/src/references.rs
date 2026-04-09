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

    let guard = Guard;
    let result = func();
    std::mem::forget(guard);
    MODULE_PATH_CONTEXT.with_borrow_mut(|ctx| {
        ctx.pop();
    });

    result
}

pub(crate) fn current_module_path() -> Option<String> {
    MODULE_PATH_CONTEXT.with_borrow(|ctx| ctx.last().cloned())
}

/// Collect all named references created in the closure body.
pub fn collect_references<R>(func: impl FnOnce() -> R) -> (R, HashSet<NamedReference>) {
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            REFERENCED_TYPES.with_borrow_mut(|types| {
                if let Some(v) = types {
                    if v.len() == 1 {
                        *types = None;
                    } else {
                        v.pop();
                    }
                }
            })
        }
    }

    REFERENCED_TYPES.with_borrow_mut(|v| {
        if let Some(v) = v {
            v.push(Default::default());
        } else {
            *v = Some(vec![Default::default()]);
        }
    });

    let guard = Guard;
    let result = func();
    std::mem::forget(guard);

    (
        result,
        REFERENCED_TYPES.with_borrow_mut(|types| {
            types
                .as_mut()
                .expect("REFERENCED_TYPES is unset but it should be set")
                .pop()
                .expect("REFERENCED_TYPES is missing a valid collection context")
        }),
    )
}

pub(crate) fn track_nr(r: &NamedReference) {
    REFERENCED_TYPES.with_borrow_mut(|ctxs| {
        if let Some(ctxs) = ctxs {
            let mut tracked = r.clone();
            tracked.generics_mut().clear();
            tracked.set_instance(None);

            for ctx in ctxs {
                ctx.insert(tracked.clone());
            }
        }
    });
}
