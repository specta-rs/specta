use specta::{SerdeError, Type};

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(export = false)]
pub struct SkipOnlyField {
    #[specta(skip)]
    a: String,
}

#[derive(Type)]
#[specta(export = false)]
pub struct SkipField {
    #[specta(skip)]
    a: String,
    b: i32,
}

#[derive(Type)]
#[specta(export = false)]
pub enum SkipOnlyVariantExternallyTagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type)]
#[specta(export = false, tag = "t")]
pub enum SkipOnlyVariantInternallyTagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type)]
#[specta(export = false, tag = "t", content = "c")]
pub enum SkipOnlyVariantAdjacentlyTagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type)]
#[specta(export = false, untagged)]
pub enum SkipOnlyVariantUntagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type)]
#[specta(export = false)]
pub enum SkipVariant {
    #[specta(skip)]
    A(String),
    B(i32),
}

#[derive(Type)]
#[specta(export = false)]
pub enum SkipUnnamedFieldInVariant {
    // only field
    A(#[specta(skip)] String),
    // not only field
    //
    // This will `B(String)` == `String` in TS whether this will be `[String]`. This is why `#[serde(skip)]` is processed at runtime not in the macro.
    B(#[specta(skip)] String, i32),
}

#[derive(Type)]
#[specta(export = false)]
pub enum SkipNamedFieldInVariant {
    // only field
    A {
        #[specta(skip)]
        a: String,
    },
    // not only field
    B {
        #[specta(skip)]
        a: String,
        b: i32,
    },
}

#[test]
fn skip() {
    assert_ts!(SkipOnlyField, "Record<string, never>");
    assert_ts!(SkipField, "{ b: number }");
    assert_ts!(error; SkipOnlyVariantExternallyTagged, SerdeError::InvalidUsageOfSkip);
    assert_ts!(error; SkipOnlyVariantInternallyTagged, SerdeError::InvalidUsageOfSkip);
    assert_ts!(error; SkipOnlyVariantAdjacentlyTagged, SerdeError::InvalidUsageOfSkip);
    assert_ts!(error; SkipOnlyVariantUntagged, SerdeError::InvalidUsageOfSkip);
    assert_ts!(SkipVariant, "{ B: number }"); // Serializing `A` will be error but that is expected behavior.
    assert_ts!(SkipUnnamedFieldInVariant, r#""A" | { B: [number] }"#);
    assert_ts!(
        SkipNamedFieldInVariant,
        "{ A: Record<string, never> } | { B: { b: number } }"
    );
}
