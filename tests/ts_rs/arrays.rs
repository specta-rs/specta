use crate::ts::assert_ts;
use specta::Type;

#[test]
fn free() {
    assert_ts!([String; 10], "string[]")
}

#[test]
fn interface() {
    #[derive(Type)]
    #[specta(export = false)]
    struct Interface {
        #[allow(dead_code)]
        a: [i32; 10],
    }

    assert_ts!(Interface, "{ a: number[] }")
}

#[test]
fn newtype() {
    #[derive(Type)]
    #[specta(export = false)]
    struct Newtype(#[allow(dead_code)] [i32; 10]);

    assert_ts!(Newtype, "number[]")
}
