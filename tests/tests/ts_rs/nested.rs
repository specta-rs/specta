use std::{cell::Cell, rc::Rc, sync::Arc};

use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
struct D {
    x1: Arc<i32>,
    y1: Cell<i32>,
}

#[derive(Type)]
#[specta(collect = false)]
struct E {
    a1: Box<D>,
    #[specta(inline)]
    a2: D,
}

#[derive(Type)]
#[specta(collect = false)]
struct F {
    b1: Rc<E>,
    #[specta(inline)]
    b2: E,
}

#[test]
fn test_nested() {
    insta::assert_snapshot!(crate::ts::inline::<F>(&Default::default()).unwrap(), @"{ b1: E; b2: { a1: D; a2: { x1: number; y1: number } } }");
}
