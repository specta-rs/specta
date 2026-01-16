use std::any::Any;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ts::assert_ts_inline2;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct SkipOnlyField {
    #[specta(skip)]
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct SkipField {
    #[specta(skip)]
    a: String,
    b: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub enum SkipOnlyVariantExternallyTagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
pub enum SkipOnlyVariantInternallyTagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
pub enum SkipOnlyVariantAdjacentlyTagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
pub enum SkipOnlyVariantUntagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub enum SkipVariant {
    #[specta(skip)]
    A(String),
    B(i32),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub enum SkipUnnamedFieldInVariant {
    // only field
    A(#[specta(skip)] String),
    // not only field
    //
    // This will `B(String)` == `String` in TS whether this will be `[String]`. This is why `#[serde(skip)]` is processed at runtime not in the macro.
    B(#[specta(skip)] String, i32),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
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

// https://github.com/specta-rs/specta/issues/170
#[derive(Type, Serialize, Deserialize)]
#[specta(transparent, collect = false)]
pub struct TransparentWithSkip((), #[specta(skip)] String);

// https://github.com/specta-rs/specta/issues/170
#[derive(Type, Serialize, Deserialize)]
#[specta(transparent, collect = false)]
pub struct TransparentWithSkip2(#[specta(skip)] (), String);

// https://github.com/specta-rs/specta/issues/170
#[derive(Type)]
#[specta(transparent, collect = false)]
pub struct TransparentWithSkip3(#[specta(type = String)] Box<dyn Any>);

/// This is intentionally just a compile or not compile test
/// https://github.com/specta-rs/specta/issues/167
#[derive(Type, Serialize)]
#[specta(collect = false)]
pub enum LazilySkip {
    #[serde(skip)]
    A(Box<dyn Any>),
    B(#[serde(skip)] Box<dyn Any>),
    C {
        #[serde(skip)]
        a: Box<dyn Any>,
    },
}

#[test]
fn skip() {
    insta::assert_snapshot!(assert_ts_inline2::<SkipOnlyField>().unwrap(), @r#"Record<string, never>"#);
    insta::assert_snapshot!(assert_ts_inline2::<SkipField>().unwrap(), @r#"{ b: number }"#);
    insta::assert_snapshot!(assert_ts_inline2::<SkipOnlyVariantExternallyTagged>().unwrap_err(), @"the usage of #[specta(skip)] means the type can't be serialized\n");
    insta::assert_snapshot!(assert_ts_inline2::<SkipOnlyVariantInternallyTagged>().unwrap_err(), @"the usage of #[specta(skip)] means the type can't be serialized\n");
    insta::assert_snapshot!(assert_ts_inline2::<SkipOnlyVariantAdjacentlyTagged>().unwrap_err(), @"the usage of #[specta(skip)] means the type can't be serialized\n");
    insta::assert_snapshot!(assert_ts_inline2::<SkipOnlyVariantUntagged>().unwrap_err(), @"the usage of #[specta(skip)] means the type can't be serialized\n");
    insta::assert_snapshot!(assert_ts_inline2::<SkipVariant>().unwrap(), @r#"{ B: number }"#); // Serializing `A` will be error but that is expected behavior.
    insta::assert_snapshot!(assert_ts_inline2::<SkipUnnamedFieldInVariant>().unwrap(), @r#""A" | { B: [number] }"#);
    insta::assert_snapshot!(assert_ts_inline2::<SkipNamedFieldInVariant>().unwrap(), @r#"{ A: Record<string, never> } | { B: { b: number } }"#);
    insta::assert_snapshot!(assert_ts_inline2::<TransparentWithSkip>().unwrap(), @r#"null"#);
    insta::assert_snapshot!(assert_ts_inline2::<TransparentWithSkip2>().unwrap(), @r#"string"#);
    insta::assert_snapshot!(assert_ts_inline2::<TransparentWithSkip3>().unwrap(), @r#"string"#);
}
