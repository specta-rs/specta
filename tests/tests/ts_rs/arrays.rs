use specta::Type;

#[test]
fn free() {
    insta::assert_snapshot!(crate::ts::inline::<[String; 3]>(&Default::default()).unwrap(), @"[string, string, string]");
}

#[test]
fn interface() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Interface {
        #[allow(dead_code)]
        a: [i32; 3],
    }

    insta::assert_snapshot!(crate::ts::inline::<Interface>(&Default::default()).unwrap(), @"{ a: [number, number, number] }");
}

#[test]
fn newtype() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Newtype(#[allow(dead_code)] [i32; 3]);

    insta::assert_snapshot!(crate::ts::inline::<Newtype>(&Default::default()).unwrap(), @"[number, number, number]");
}
