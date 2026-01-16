use specta::Type;

#[test]
fn list() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct List {
        #[allow(dead_code)]
        data: Option<Vec<u32>>,
    }

    insta::assert_snapshot!(crate::ts::inline::<List>(&Default::default()).unwrap(), @"{ data: number[] | null }");
}
