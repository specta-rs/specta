use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
struct NonOptional(Option<String>);

#[derive(Type)]
#[specta(collect = false)]
struct OptionalOnNamedField(#[specta(optional)] Option<String>); // Should do nothing

#[derive(Type)]
#[specta(collect = false, transparent, inline)]
struct OptionalOnTransparentNamedFieldInner(#[specta(optional)] Option<String>);

#[derive(Type)]
#[specta(collect = false)]
struct OptionalOnTransparentNamedField {
    // Now it should work
    b: OptionalOnTransparentNamedFieldInner,
}

#[derive(Type)]
#[specta(collect = false)]
enum OptionalInEnum {
    // Should do nothing
    A(#[specta(optional)] Option<String>),
    // Base case without `optional`
    B {
        a: Option<String>,
    },
    // Should add `?` on field
    C {
        #[specta(optional)]
        a: Option<String>,
    },
}

#[test]
fn optional() {
    insta::assert_snapshot!(crate::ts::inline::<NonOptional>(&Default::default()).unwrap(), @"string | null");
    insta::assert_snapshot!(crate::ts::inline::<OptionalOnNamedField>(&Default::default()).unwrap(), @"string | null");
    // assert_ts!(OptionalOnTransparentNamedField, "{ b?: string | null }"); // TODO: Fix this
    insta::assert_snapshot!(crate::ts::inline::<OptionalInEnum>(&Default::default()).unwrap(), @"{ A: string | null } | { B: { a: string | null } } | { C: { a?: string | null } }");
}
