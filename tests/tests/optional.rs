use specta::Type;

use crate::ts::assert_ts;

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
    assert_ts!(NonOptional, "string | null");
    assert_ts!(OptionalOnNamedField, "string | null");
    // assert_ts!(OptionalOnTransparentNamedField, "{ b?: string | null }"); // TODO: Fix this
    assert_ts!(
        OptionalInEnum,
        "{ A: string | null } | { B: { a: string | null } } | { C: { a?: string | null } }"
    );
}
