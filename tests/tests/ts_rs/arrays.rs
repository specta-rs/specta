use crate::ts::assert_ts;
use specta::Type;

#[test]
fn free() {
    assert_ts!([String; 3], "[string, string, string]")
}

#[test]
fn interface() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Interface {
        #[allow(dead_code)]
        a: [i32; 3],
    }

    assert_ts!(Interface, "{ a: [number, number, number] }")
}

#[test]
fn newtype() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Newtype(#[allow(dead_code)] [i32; 3]);

    assert_ts!(Newtype, "[number, number, number]")
}
