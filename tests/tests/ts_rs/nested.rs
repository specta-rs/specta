use std::{cell::Cell, rc::Rc, sync::Arc};

use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(export = false)]
struct D {
    x1: Arc<i32>,
    y1: Cell<i32>,
}

#[derive(Type)]
#[specta(export = false)]
struct E {
    a1: Box<D>,
    #[specta(inline)]
    a2: D,
}

#[derive(Type)]
#[specta(export = false)]
struct F {
    b1: Rc<E>,
    #[specta(inline)]
    b2: E,
}

#[test]
fn test_nested() {
    assert_ts!(
        F,
        "{ b1: E; b2: { a1: D; a2: { x1: number; y1: number } } }"
    );
}
