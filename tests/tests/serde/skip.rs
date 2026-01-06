use std::any::Any;

use specta::Type;

use crate::ts::assert_ts_inline2;

#[derive(Type)]
#[specta(collect = false)]
pub struct SkipOnlyField {
    #[specta(skip)]
    a: String,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct SkipField {
    #[specta(skip)]
    a: String,
    b: i32,
}

#[derive(Type)]
#[specta(collect = false)]
pub enum SkipOnlyVariantExternallyTagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "t")]
pub enum SkipOnlyVariantInternallyTagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
pub enum SkipOnlyVariantAdjacentlyTagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type)]
#[specta(collect = false)]
#[serde(untagged)]
pub enum SkipOnlyVariantUntagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type)]
#[specta(collect = false)]
pub enum SkipVariant {
    #[specta(skip)]
    A(String),
    B(i32),
}

#[derive(Type)]
#[specta(collect = false)]
pub enum SkipUnnamedFieldInVariant {
    // only field
    A(#[specta(skip)] String),
    // not only field
    //
    // This will `B(String)` == `String` in TS whether this will be `[String]`. This is why `#[serde(skip)]` is processed at runtime not in the macro.
    B(#[specta(skip)] String, i32),
}

#[derive(Type)]
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

// https://github.com/oscartbeaumont/specta/issues/170
#[derive(Type)]
#[specta(transparent, collect = false)]
pub struct TransparentWithSkip((), #[specta(skip)] String);

// https://github.com/oscartbeaumont/specta/issues/170
#[derive(Type)]
#[specta(transparent, collect = false)]
pub struct TransparentWithSkip2(#[specta(skip)] (), String);

// https://github.com/oscartbeaumont/specta/issues/170
#[derive(Type)]
#[specta(transparent, collect = false)]
pub struct TransparentWithSkip3(#[specta(type = String)] Box<dyn Any>);

/// This is intentionally just a compile or not compile test
/// https://github.com/oscartbeaumont/specta/issues/167
#[derive(Type)]
#[specta(collect = false)]
pub enum LazilySkip {
    #[specta(skip)]
    A(Box<dyn Any>),
    B(#[specta(skip)] Box<dyn Any>),
    C {
        #[specta(skip)]
        a: Box<dyn Any>,
    },
}

#[test]
fn skip() {
    assert_eq!(
        assert_ts_inline2::<SkipOnlyField>(),
        Ok(r#"Record<string, never>"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<SkipField>(),
        Ok(r#"{ b: number }"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<SkipOnlyVariantExternallyTagged>(),
        Err("the usage of #[specta(skip)] means the type can't be serialized\n".into())
    );
    assert_eq!(
        assert_ts_inline2::<SkipOnlyVariantInternallyTagged>(),
        Err("the usage of #[specta(skip)] means the type can't be serialized\n".into())
    );
    assert_eq!(
        assert_ts_inline2::<SkipOnlyVariantAdjacentlyTagged>(),
        Err("the usage of #[specta(skip)] means the type can't be serialized\n".into())
    );
    assert_eq!(
        assert_ts_inline2::<SkipOnlyVariantUntagged>(),
        Err("the usage of #[specta(skip)] means the type can't be serialized\n".into())
    );
    assert_eq!(
        assert_ts_inline2::<SkipVariant>(),
        Ok(r#"{ B: number }"#.into())
    ); // Serializing `A` will be error but that is expected behavior.
    assert_eq!(
        assert_ts_inline2::<SkipUnnamedFieldInVariant>(),
        Ok(r#""A" | { B: [number] }"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<SkipNamedFieldInVariant>(),
        Ok(r#"{ A: Record<string, never> } | { B: { b: number } }"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<TransparentWithSkip>(),
        Ok(r#"null"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<TransparentWithSkip2>(),
        Ok(r#"string"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<TransparentWithSkip3>(),
        Ok(r#"string"#.into())
    );
}
