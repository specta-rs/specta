// Regression test: `#[serde(flatten)]` on an `Option<T>` field.
//
// Serde contributes nothing when the flattened `Option<T>` is `None`, and
// merges `T`'s fields when it's `Some`. Previously `specta-serde` pushed the
// field's `Nullable(T)` type verbatim into the intersection it built for the
// flattened struct/variant, producing `Base & T | null`. Because TypeScript's
// `&` binds tighter than `|`, that parses as `(Base & T) | null` which is
// wrong in both directions: the wire value is never bare `null`, and the
// legitimate `None` output (base fields only) doesn't satisfy `Base & T`.

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Inner1 {
    x: i32,
}

impl Inner1 {
    fn is_zero(&self) -> bool {
        self.x == 0
    }
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Inner2 {
    y: bool,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlatOpt {
    a: i32,
    #[serde(flatten)]
    inner: Option<Inner1>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlatConditional {
    a: i32,
    #[serde(flatten, skip_serializing_if = "Inner1::is_zero")]
    inner: Inner1,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum InternalFlattenOpt {
    A {
        #[serde(flatten)]
        inner: Option<Inner1>,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum InternalFlattenConditional {
    A {
        #[serde(flatten, skip_serializing_if = "Inner1::is_zero")]
        inner: Inner1,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum InternalFlattenConditionalAlias {
    A {
        #[serde(alias = "old_value")]
        value: String,
        #[serde(flatten, skip_serializing_if = "Inner1::is_zero")]
        inner: Inner1,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasFlattenInner {
    old_value: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasFlattenCollision {
    #[serde(alias = "old_value")]
    value: String,
    #[serde(flatten)]
    inner: AliasFlattenInner,
}

#[derive(Clone, Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasedFlattenPayload {
    #[serde(alias = "outer_key")]
    value: String,
}

#[derive(Clone, Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(into = "AliasedFlattenPayload", from = "AliasedFlattenPayload")]
struct ConvertedAliasedFlattenPayload {
    value: String,
}

impl From<ConvertedAliasedFlattenPayload> for AliasedFlattenPayload {
    fn from(value: ConvertedAliasedFlattenPayload) -> Self {
        Self { value: value.value }
    }
}

impl From<AliasedFlattenPayload> for ConvertedAliasedFlattenPayload {
    fn from(value: AliasedFlattenPayload) -> Self {
        Self { value: value.value }
    }
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentKeyCollidesWithConvertedFlattenedAlias {
    outer_key: String,
    #[serde(flatten)]
    inner: ConvertedAliasedFlattenPayload,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentKeyCollidesThroughConvertedFlattenedWrapper {
    outer_key: String,
    #[serde(flatten)]
    wrapper: FlattenWrapper<ConvertedAliasedFlattenPayload>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentKeyCollidesWithFlattenedAlias {
    outer_key: String,
    #[serde(flatten)]
    inner: AliasedFlattenPayload,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct RecursiveFlattenedAlias {
    #[serde(alias = "old_value")]
    value: String,
    #[serde(flatten)]
    next: Option<Box<RecursiveFlattenedAlias>>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct UnrelatedFlattenedPayload {
    y: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasWithUnrelatedFlatten {
    #[serde(alias = "old_x")]
    x: String,
    #[serde(flatten)]
    payload: UnrelatedFlattenedPayload,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenedOldKey {
    old: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkippedFlattenedOldKey {
    #[serde(skip_serializing, skip_deserializing)]
    old: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasBesideSkippedFlattenedKey {
    #[serde(alias = "old")]
    value: String,
    #[serde(flatten)]
    flattened: SkippedFlattenedOldKey,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct MultipleAliasesWithOneFlattenCollision {
    #[serde(alias = "old", alias = "legacy")]
    value: String,
    #[serde(flatten)]
    flattened: FlattenedOldKey,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenedFormerSentinelKey {
    #[serde(rename = "specta_serde:any_flattened_key")]
    key: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasesBesideFormerSentinelKey {
    #[serde(alias = "old", alias = "legacy")]
    value: String,
    #[serde(flatten)]
    flattened: FlattenedFormerSentinelKey,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasWithFlattenedMap {
    #[serde(alias = "old_value")]
    value: String,
    #[serde(flatten)]
    rest: std::collections::HashMap<String, String>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum FlattenedExternalTag {
    Kind { payload: String },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum FlattenedVariantUntagged {
    #[serde(untagged)]
    Value { old: String },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasBesideFlattenedVariantUntagged {
    #[serde(alias = "old")]
    value: String,
    #[serde(flatten)]
    untagged: FlattenedVariantUntagged,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasBesideFlattenedExternalTag {
    #[serde(alias = "Kind")]
    value: String,
    #[serde(flatten)]
    tagged: FlattenedExternalTag,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct NestedAliasedObject {
    #[serde(alias = "outer_key")]
    value: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenedPayloadWithNestedAlias {
    nested: NestedAliasedObject,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct OuterKeyBesideNestedAlias {
    outer_key: String,
    #[serde(flatten)]
    payload: FlattenedPayloadWithNestedAlias,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum AliasedFlattenEnumPayload {
    Value {
        #[serde(alias = "outer_key")]
        value: String,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum FlattenedUntaggedNewtypeAlias {
    Value(AliasedFlattenPayload),
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum FlattenedExternalNewtypeAlias {
    Value(AliasedFlattenPayload),
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentKeyCollidesThroughUntaggedNewtype {
    outer_key: String,
    #[serde(flatten)]
    inner: FlattenedUntaggedNewtypeAlias,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentKeyBesideExternalNewtype {
    outer_key: String,
    #[serde(flatten)]
    inner: FlattenedExternalNewtypeAlias,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
struct TaggedFlattenPayload {
    value: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasBesideTaggedFlattenPayload {
    #[serde(alias = "kind")]
    canonical: String,
    #[serde(flatten)]
    inner: TaggedFlattenPayload,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenNewtype(AliasedFlattenPayload);

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentKeyCollidesThroughNewtypeStruct {
    outer_key: String,
    #[serde(flatten)]
    inner: FlattenNewtype,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct PairFlattenWrapper<A, B> {
    #[serde(skip)]
    marker: std::marker::PhantomData<A>,
    #[serde(flatten)]
    inner: B,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentThroughNestedPairWrapper {
    outer_key: String,
    #[serde(flatten)]
    inner: PairFlattenWrapper<String, PairFlattenWrapper<String, AliasedFlattenPayload>>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentAliasThroughDoubleFlattenWrapper {
    #[serde(alias = "value")]
    canonical: String,
    #[serde(flatten)]
    inner: FlattenWrapper<FlattenWrapper<AliasedFlattenPayload>>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "tag", content = "payload")]
enum AdjacentWithSkippedContent {
    Unit,
    #[serde(skip)]
    Hidden {
        value: String,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasBesideAdjacentWithoutContent {
    #[serde(alias = "payload")]
    canonical: String,
    #[serde(flatten)]
    inner: AdjacentWithSkippedContent,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "tag", content = "content")]
enum AdjacentWithSerializeSkippedNewtype {
    Value(#[serde(skip_serializing)] String),
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasBesidePhasedAdjacentContent {
    #[serde(alias = "content")]
    canonical: String,
    #[serde(flatten)]
    inner: AdjacentWithSerializeSkippedNewtype,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedWithSkippedCollisionKey {
    Live {
        other: String,
    },
    #[serde(skip)]
    Hidden {
        inactive_key: String,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasBesideSkippedUntaggedVariant {
    #[serde(alias = "inactive_key")]
    canonical: String,
    #[serde(flatten)]
    inner: UntaggedWithSkippedCollisionKey,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedWithPhaseSkippedCollisionKey {
    Live {
        other: String,
    },
    #[serde(skip_serializing)]
    DeserializeOnly {
        phased_key: String,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasBesidePhasedSkippedUntaggedVariant {
    #[serde(alias = "phased_key")]
    canonical: String,
    #[serde(flatten)]
    inner: UntaggedWithPhaseSkippedCollisionKey,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
struct TaggedStructWithTagAlias {
    #[serde(alias = "kind")]
    value: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false, transparent = false)]
#[serde(transparent)]
struct TransparentNamedFlattenWrapper {
    payload: AliasedFlattenPayload,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentKeyCollidesThroughTransparentNamedStruct {
    outer_key: String,
    #[serde(flatten)]
    inner: TransparentNamedFlattenWrapper,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkippedTupleFlattenWrapper(#[serde(skip)] String, AliasedFlattenPayload);

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentKeyBesideSkippedTupleStruct {
    outer_key: String,
    #[serde(flatten)]
    inner: SkippedTupleFlattenWrapper,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum EnumVariantWithFlattenedAlias {
    Value {
        outer_key: String,
        #[serde(flatten)]
        inner: AliasedFlattenPayload,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentKeyCollidesWithFlattenedEnumAlias {
    outer_key: String,
    #[serde(flatten)]
    inner: AliasedFlattenEnumPayload,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenWrapper<T> {
    #[serde(flatten)]
    inner: T,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct PlainFlattenPayload {
    plain: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct DuoFlattenWrapper<A, B> {
    #[serde(flatten)]
    first: FlattenWrapper<A>,
    #[serde(flatten)]
    second: FlattenWrapper<B>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentKeyCollidesInSecondGenericInstantiation {
    outer_key: String,
    #[serde(flatten)]
    duo: DuoFlattenWrapper<PlainFlattenPayload, AliasedFlattenPayload>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentKeyCollidesThroughFlattenedWrapper {
    outer_key: String,
    #[serde(flatten)]
    wrapper: FlattenWrapper<AliasedFlattenPayload>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentKeyCollidesThroughDoubleFlattenedWrapper {
    outer_key: String,
    #[serde(flatten)]
    wrapper: FlattenWrapper<FlattenWrapper<AliasedFlattenPayload>>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ParentAliasCollidesThroughFlattenedWrapper {
    #[serde(alias = "value")]
    outer_key: String,
    #[serde(flatten)]
    wrapper: FlattenWrapper<AliasedFlattenPayload>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum InternalTagWithFlattenedAliasCollision {
    Value {
        outer_key: String,
        #[serde(flatten)]
        inner: AliasedFlattenPayload,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenedSiblingAlias {
    #[serde(alias = "x")]
    value: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenedSiblingKey {
    x: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum InternalTagWithSiblingFlattenCollision {
    Value {
        #[serde(flatten)]
        aliased: FlattenedSiblingAlias,
        #[serde(flatten)]
        keyed: FlattenedSiblingKey,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct InternalTagAliasPayload {
    #[serde(alias = "t")]
    value: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum InternalTagNewtypeAliasCollision {
    Value(InternalTagAliasPayload),
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlatTwoOpt {
    a: i32,
    #[serde(flatten)]
    inner1: Option<Inner1>,
    #[serde(flatten)]
    inner2: Option<Inner2>,
}

#[derive(Debug, Default, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(default)]
struct DefaultInner {
    z: i32,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlatDefault {
    a: i32,
    #[serde(flatten)]
    inner: DefaultInner,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlatNestedOpt {
    a: i32,
    #[serde(flatten)]
    inner: Option<Option<Inner1>>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Inner3 {
    z: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlatThreeOpt {
    a: i32,
    #[serde(flatten)]
    inner1: Option<Inner1>,
    #[serde(flatten)]
    inner2: Option<Inner2>,
    #[serde(flatten)]
    inner3: Option<Inner3>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Req {
    r: String,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlatMixed {
    a: i32,
    #[serde(flatten)]
    req: Req,
    #[serde(flatten)]
    opt: Option<Inner1>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum TagWithFields {
    A {
        b: bool,
        #[serde(flatten)]
        inner: Option<Inner1>,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlatOptMap {
    a: i32,
    #[serde(flatten)]
    rest: Option<std::collections::HashMap<String, i32>>,
}

// --- serde_json evidence, so the exported type can be checked against what
// serde actually puts on the wire (rather than against assumptions). ---

#[test]
fn flat_opt_serde_evidence() {
    let none = FlatOpt { a: 1, inner: None };
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"a":1}"#);

    let some = FlatOpt {
        a: 1,
        inner: Some(Inner1 { x: 2 }),
    };
    assert_eq!(serde_json::to_string(&some).unwrap(), r#"{"a":1,"x":2}"#);

    let de_none: FlatOpt = serde_json::from_str(r#"{"a":1}"#).unwrap();
    assert!(de_none.inner.is_none());

    let de_some: FlatOpt = serde_json::from_str(r#"{"a":1,"x":2}"#).unwrap();
    assert_eq!(de_some.inner.unwrap().x, 2);
}

#[test]
fn conditional_flatten_serde_evidence() {
    assert_eq!(
        serde_json::to_string(&FlatConditional {
            a: 1,
            inner: Inner1 { x: 0 },
        })
        .unwrap(),
        r#"{"a":1}"#,
    );
    assert_eq!(
        serde_json::to_string(&InternalFlattenConditional::A {
            inner: Inner1 { x: 0 },
        })
        .unwrap(),
        r#"{"t":"A"}"#,
    );
    assert_eq!(
        serde_json::to_string(&InternalFlattenConditionalAlias::A {
            value: "current".into(),
            inner: Inner1 { x: 0 },
        })
        .unwrap(),
        r#"{"t":"A","value":"current"}"#,
    );
}

#[test]
fn internal_flatten_opt_serde_evidence() {
    let none = InternalFlattenOpt::A { inner: None };
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"t":"A"}"#);

    let some = InternalFlattenOpt::A {
        inner: Some(Inner1 { x: 1 }),
    };
    assert_eq!(serde_json::to_string(&some).unwrap(), r#"{"t":"A","x":1}"#);

    let de_none: InternalFlattenOpt = serde_json::from_str(r#"{"t":"A"}"#).unwrap();
    match de_none {
        InternalFlattenOpt::A { inner } => assert!(inner.is_none()),
    }

    let de_some: InternalFlattenOpt = serde_json::from_str(r#"{"t":"A","x":1}"#).unwrap();
    match de_some {
        InternalFlattenOpt::A { inner } => assert_eq!(inner.unwrap().x, 1),
    }
}

#[test]
fn flat_two_opt_serde_evidence() {
    let both_none = FlatTwoOpt {
        a: 1,
        inner1: None,
        inner2: None,
    };
    assert_eq!(serde_json::to_string(&both_none).unwrap(), r#"{"a":1}"#);

    let only_inner1 = FlatTwoOpt {
        a: 1,
        inner1: Some(Inner1 { x: 2 }),
        inner2: None,
    };
    assert_eq!(
        serde_json::to_string(&only_inner1).unwrap(),
        r#"{"a":1,"x":2}"#
    );

    let only_inner2 = FlatTwoOpt {
        a: 1,
        inner1: None,
        inner2: Some(Inner2 { y: true }),
    };
    assert_eq!(
        serde_json::to_string(&only_inner2).unwrap(),
        r#"{"a":1,"y":true}"#
    );

    let both_some = FlatTwoOpt {
        a: 1,
        inner1: Some(Inner1 { x: 2 }),
        inner2: Some(Inner2 { y: true }),
    };
    assert_eq!(
        serde_json::to_string(&both_some).unwrap(),
        r#"{"a":1,"x":2,"y":true}"#
    );
}

#[test]
fn flat_default_serde_evidence() {
    // A `#[serde(flatten, default)]` non-`Option` field is unaffected by this
    // fix: its type is never `Nullable`, so it stays a plain (mandatory)
    // intersection part.
    let de: FlatDefault = serde_json::from_str(r#"{"a":1}"#).unwrap();
    assert_eq!(de.a, 1);
    assert_eq!(de.inner.z, 0);

    let with_inner: FlatDefault = serde_json::from_str(r#"{"a":1,"z":5}"#).unwrap();
    assert_eq!(with_inner.inner.z, 5);
}

#[test]
fn flat_nested_opt_serde_evidence() {
    // `Option<Option<T>>` flattens the same way `Option<T>` does.
    let none = FlatNestedOpt { a: 1, inner: None };
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"a":1}"#);

    let some_none = FlatNestedOpt {
        a: 1,
        inner: Some(None),
    };
    assert_eq!(serde_json::to_string(&some_none).unwrap(), r#"{"a":1}"#);

    let some_some = FlatNestedOpt {
        a: 1,
        inner: Some(Some(Inner1 { x: 2 })),
    };
    assert_eq!(
        serde_json::to_string(&some_some).unwrap(),
        r#"{"a":1,"x":2}"#
    );

    // Deserialization never distinguishes `Some(None)` from `None` on the
    // wire either - both come from the same base-only shape.
    let de_absent: FlatNestedOpt = serde_json::from_str(r#"{"a":1}"#).unwrap();
    assert!(matches!(de_absent.inner, Some(None)));

    let de_present: FlatNestedOpt = serde_json::from_str(r#"{"a":1,"x":2}"#).unwrap();
    assert_eq!(de_present.inner.unwrap().unwrap().x, 2);
}

#[test]
fn flat_mixed_serde_evidence() {
    // A mandatory flattened struct always contributes its fields; the
    // flattened `Option` still contributes only when `Some`.
    let none = FlatMixed {
        a: 1,
        req: Req { r: "s".into() },
        opt: None,
    };
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"a":1,"r":"s"}"#);

    let some = FlatMixed {
        a: 1,
        req: Req { r: "s".into() },
        opt: Some(Inner1 { x: 2 }),
    };
    assert_eq!(
        serde_json::to_string(&some).unwrap(),
        r#"{"a":1,"r":"s","x":2}"#
    );

    let de: FlatMixed = serde_json::from_str(r#"{"a":1,"r":"s"}"#).unwrap();
    assert!(de.opt.is_none());
}

#[test]
fn tag_with_fields_serde_evidence() {
    let none = TagWithFields::A {
        b: true,
        inner: None,
    };
    assert_eq!(
        serde_json::to_string(&none).unwrap(),
        r#"{"t":"A","b":true}"#
    );

    let some = TagWithFields::A {
        b: true,
        inner: Some(Inner1 { x: 2 }),
    };
    assert_eq!(
        serde_json::to_string(&some).unwrap(),
        r#"{"t":"A","b":true,"x":2}"#
    );

    let de: TagWithFields = serde_json::from_str(r#"{"t":"A","b":true}"#).unwrap();
    match de {
        TagWithFields::A { inner, .. } => assert!(inner.is_none()),
    }
}

#[test]
fn flat_opt_map_serde_evidence() {
    // Serde also allows flattening a map (as a catch-all for leftover keys).
    let none = FlatOptMap { a: 1, rest: None };
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"a":1}"#);

    let some = FlatOptMap {
        a: 1,
        rest: Some([("k".to_string(), 9)].into_iter().collect()),
    };
    assert_eq!(serde_json::to_string(&some).unwrap(), r#"{"a":1,"k":9}"#);

    // Deserialization of a flattened `Option<Map>` is always `Some` (possibly
    // empty) because an empty leftover set is still a valid map - but the
    // *wire shapes* are the same as any other flattened Option: base-only or
    // base plus entries. The exported union covers exactly those.
    let de: FlatOptMap = serde_json::from_str(r#"{"a":1}"#).unwrap();
    assert_eq!(de.rest, Some(Default::default()));
}

// --- Exported TypeScript, checked against the serde evidence above. ---

#[test]
fn flat_opt_exports_union_instead_of_nullable_intersection() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FlatOpt>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        !rendered.contains("| null"),
        "flattened Option must not admit bare `null`:\n{rendered}"
    );

    let expected = "export type FlatOpt = {\n\ta: number,\n} & Inner1 | {\n\ta: number,\n};";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[test]
fn conditional_flatten_exports_optional_union() {
    let types = Types::default()
        .register::<FlatConditional>()
        .register::<InternalFlattenConditional>()
        .register::<InternalFlattenConditionalAlias>();
    let rendered = Typescript::default()
        .export(&types, specta_serde::Format)
        .expect("export should succeed");

    for expected in [
        "export type FlatConditional = {\n\ta: number,\n} & Inner1 | {\n\ta: number,\n};",
        "export type InternalFlattenConditional = {\n\tt: \"A\",\n} & Inner1 | {\n\tt: \"A\",\n};",
        "export type InternalFlattenConditionalAlias = {\n\tt: \"A\",\n} & ({\n\tvalue: string,\n\told_value?: never,\n} | {\n\told_value: string,\n\tvalue?: never,\n}) & Inner1 | {\n\tt: \"A\",\n} & ({\n\tvalue: string,\n\told_value?: never,\n} | {\n\told_value: string,\n\tvalue?: never,\n});",
    ] {
        assert!(
            rendered.contains(expected),
            "expected:\n{expected}\n\ngot:\n{rendered}"
        );
    }

    let phased = Typescript::default()
        .export(&types, specta_serde::PhasesFormat)
        .expect("phased export should succeed");
    for expected in [
        "export type FlatConditional_Serialize = {\n\ta: number,\n} & Inner1 | {\n\ta: number,\n};",
        "export type FlatConditional_Deserialize = {\n\ta: number,\n} & Inner1;",
        "export type InternalFlattenConditional_Serialize = {\n\tt: \"A\",\n} & Inner1 | {\n\tt: \"A\",\n};",
        "export type InternalFlattenConditional_Deserialize = {\n\tt: \"A\",\n} & Inner1;",
        "export type InternalFlattenConditionalAlias_Serialize = {\n\tt: \"A\",\n} & {\n\tvalue: string,\n} & Inner1 | {\n\tt: \"A\",\n} & {\n\tvalue: string,\n};",
        "export type InternalFlattenConditionalAlias_Deserialize = {\n\tt: \"A\",\n} & ({\n\tvalue: string,\n\told_value?: never,\n} | {\n\told_value: string,\n\tvalue?: never,\n}) & Inner1;",
    ] {
        assert!(
            phased.contains(expected),
            "expected:\n{expected}\n\ngot:\n{phased}"
        );
    }
}

#[test]
fn flattened_keys_do_not_become_alias_exclusions() {
    let value = AliasFlattenCollision {
        value: "canonical".into(),
        inner: AliasFlattenInner {
            old_value: "flattened".into(),
        },
    };
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::json!({ "value": "canonical", "old_value": "flattened" })
    );

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<AliasFlattenCollision>(),
            specta_serde::Format,
        )
        .expect("flattened aliases should export without excluding flattened keys");
    assert!(!rendered.contains("old_value?: never"), "{rendered}");
    assert!(rendered.contains("value?: never"), "{rendered}");
}

#[test]
fn aliases_inside_flattened_enum_payloads_do_not_exclude_parent_keys() {
    let value = ParentKeyCollidesWithFlattenedEnumAlias {
        outer_key: "outer".into(),
        inner: AliasedFlattenEnumPayload::Value {
            value: "inner".into(),
        },
    };
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::json!({ "outer_key": "outer", "value": "inner" })
    );

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentKeyCollidesWithFlattenedEnumAlias>(),
            specta_serde::Format,
        )
        .expect("flattened enum aliases should not exclude parent keys");
    assert!(
        rendered.contains(
            "export type AliasedFlattenEnumPayload = {\n\tvalue: string,\n\touter_key?: never,\n} | {\n\touter_key: string,\n\tvalue?: never,\n};"
        ),
        "ordinary enum uses must retain alias exclusions:\n{rendered}"
    );
    let parent = rendered
        .split_once("export type ParentKeyCollidesWithFlattenedEnumAlias = ")
        .expect("parent enum type should be exported")
        .1;
    assert!(
        !parent.contains("outer_key?: never"),
        "the flattened enum use must relax alias exclusions:\n{rendered}"
    );
}

#[test]
fn aliases_through_flattened_untagged_newtypes_are_relaxed() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentKeyCollidesThroughUntaggedNewtype>(),
            specta_serde::Format,
        )
        .expect("flattened untagged newtype aliases should export");
    let parent = rendered
        .split_once("export type ParentKeyCollidesThroughUntaggedNewtype = ")
        .expect("untagged-newtype parent should be exported")
        .1;

    assert!(
        !parent.contains("outer_key?: never"),
        "the untagged newtype payload alias must be relaxed:\n{rendered}"
    );
}

#[test]
fn aliases_inside_flattened_external_newtypes_remain_exclusive() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentKeyBesideExternalNewtype>(),
            specta_serde::Format,
        )
        .expect("flattened externally tagged newtype aliases should export");
    assert!(
        rendered.contains(
            "export type AliasedFlattenPayload = {\n\tvalue: string,\n\touter_key?: never,"
        ),
        "the nested payload alias must remain exclusive:\n{rendered}"
    );
    assert!(
        rendered.contains("} & FlattenedExternalNewtypeAlias;"),
        "the externally tagged enum must remain nested:\n{rendered}"
    );
}

#[test]
fn generated_flattened_struct_tags_relax_alias_exclusions() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<AliasBesideTaggedFlattenPayload>(),
            specta_serde::Format,
        )
        .expect("tagged flattened struct aliases should export");
    let parent = rendered
        .split_once("export type AliasBesideTaggedFlattenPayload = ")
        .expect("tagged flattened parent should be exported")
        .1;

    assert!(
        !parent.contains("kind?: never"),
        "the generated flattened tag must relax the parent alias exclusion:\n{rendered}"
    );
}

#[test]
fn flattened_newtype_struct_payload_keys_relax_alias_exclusions() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentKeyCollidesThroughNewtypeStruct>(),
            specta_serde::Format,
        )
        .expect("flattened newtype struct aliases should export");
    let parent = rendered
        .split_once("export type ParentKeyCollidesThroughNewtypeStruct = ")
        .expect("newtype struct parent should be exported")
        .1;

    assert!(
        !parent.contains("outer_key?: never"),
        "the newtype payload key must relax the parent alias exclusion:\n{rendered}"
    );
}

#[test]
fn unchanged_generic_arguments_do_not_block_finite_wrapper_descent() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentThroughNestedPairWrapper>(),
            specta_serde::Format,
        )
        .expect("nested multi-parameter wrappers should export");
    let parent = rendered
        .split_once("export type ParentThroughNestedPairWrapper = ")
        .expect("nested pair-wrapper parent should be exported")
        .1;

    assert!(
        !parent.contains("outer_key?: never"),
        "an unchanged generic argument must not hide the nested alias:\n{rendered}"
    );
}

#[test]
fn flattened_key_collection_tracks_nested_generic_instantiations() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentAliasThroughDoubleFlattenWrapper>(),
            specta_serde::Format,
        )
        .expect("parent aliases through nested wrappers should export");
    let parent = rendered
        .split_once("export type ParentAliasThroughDoubleFlattenWrapper = ")
        .expect("nested-wrapper alias parent should be exported")
        .1;

    assert!(
        !parent.contains("value?: never"),
        "nested wrapper keys must relax the parent alias exclusion:\n{rendered}"
    );
}

#[test]
fn skipped_adjacent_variants_do_not_contribute_content_keys() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<AliasBesideAdjacentWithoutContent>(),
            specta_serde::Format,
        )
        .expect("adjacent enum with skipped content should export");
    let parent = rendered
        .split_once("export type AliasBesideAdjacentWithoutContent = ")
        .expect("adjacent parent should be exported")
        .1;

    assert!(
        parent.contains("payload?: never"),
        "skipped variants must not relax the content-key exclusion:\n{rendered}"
    );
}

#[test]
fn skipped_adjacent_newtype_content_is_collected_per_phase() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<AliasBesidePhasedAdjacentContent>(),
            specta_serde::PhasesFormat,
        )
        .expect("phase-specific adjacent content should export");
    let serialize = rendered
        .split_once("export type AliasBesidePhasedAdjacentContent_Serialize = ")
        .expect("serialize shape should be exported")
        .1;
    let deserialize = rendered
        .split_once("export type AliasBesidePhasedAdjacentContent_Deserialize = ")
        .expect("deserialize shape should be exported")
        .1
        .split_once("\n\n")
        .map_or_else(|| rendered.as_str(), |(shape, _)| shape);

    assert!(
        !serialize.contains("content: string"),
        "serialize omits the skipped newtype content key:\n{rendered}"
    );
    assert!(
        !deserialize.contains("content?: never"),
        "deserialize accepts the adjacent content key:\n{rendered}"
    );
}

#[test]
fn skipped_untagged_variants_do_not_contribute_flattened_keys() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<AliasBesideSkippedUntaggedVariant>(),
            specta_serde::Format,
        )
        .expect("untagged enum with a skipped variant should export");
    let parent = rendered
        .split_once("export type AliasBesideSkippedUntaggedVariant = ")
        .expect("untagged parent should be exported")
        .1;

    assert!(
        parent.contains("inactive_key?: never"),
        "a skipped variant key must not relax the alias exclusion:\n{rendered}"
    );
}

#[test]
fn phase_skipped_untagged_variants_do_not_contribute_flattened_keys() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<AliasBesidePhasedSkippedUntaggedVariant>(),
            specta_serde::PhasesFormat,
        )
        .expect("phase-skipped untagged variant should export");
    let deserialize = rendered
        .split_once("export type AliasBesidePhasedSkippedUntaggedVariant_Deserialize = ")
        .expect("deserialize parent should be exported")
        .1
        .split_once("\n\n")
        .expect("deserialize declaration should terminate")
        .0;
    let serialize = rendered
        .split_once("export type UntaggedWithPhaseSkippedCollisionKey_Serialize = ")
        .expect("serialize enum should be exported")
        .1;

    assert!(
        !deserialize.contains("phased_key?: never"),
        "the deserialize-active key must relax alias exclusivity:\n{rendered}"
    );
    assert!(
        !serialize.contains("phased_key: string"),
        "the serialize phase must omit the skipped variant:\n{rendered}"
    );
}

#[test]
fn generated_struct_tags_relax_field_alias_exclusions() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<TaggedStructWithTagAlias>(),
            specta_serde::Format,
        )
        .expect("tagged struct with a tag alias should export");
    let tagged = rendered
        .split_once("export type TaggedStructWithTagAlias = ")
        .expect("tagged struct should be exported")
        .1;

    assert!(
        !tagged.contains("kind?: never"),
        "the generated tag must relax the colliding alias exclusion:\n{rendered}"
    );
    assert!(
        tagged.contains("kind: string"),
        "serde accepts the generated tag as the aliased field spelling:\n{rendered}"
    );
    assert_eq!(
        serde_json::to_value(TaggedStructWithTagAlias {
            value: "value".into(),
        })
        .unwrap(),
        serde_json::json!({ "kind": "TaggedStructWithTagAlias", "value": "value" })
    );
    assert_eq!(
        serde_json::from_value::<TaggedStructWithTagAlias>(
            serde_json::json!({ "kind": "TaggedStructWithTagAlias" })
        )
        .unwrap()
        .value,
        "TaggedStructWithTagAlias",
        "serde exposes the generated tag through the matching field alias"
    );
}

#[test]
fn transparent_named_payload_keys_relax_alias_exclusions() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentKeyCollidesThroughTransparentNamedStruct>(),
            specta_serde::Format,
        )
        .expect("transparent named flattened aliases should export");
    let parent = rendered
        .split_once("export type ParentKeyCollidesThroughTransparentNamedStruct = ")
        .expect("transparent named parent should be exported")
        .1;

    assert!(
        !parent.contains("outer_key?: never"),
        "the transparent payload key must relax the parent alias exclusion:\n{rendered}"
    );
}

#[test]
fn skipped_tuple_fields_do_not_turn_tuple_structs_into_newtypes() {
    let error = Typescript::default()
        .export(
            &Types::default().register::<ParentKeyBesideSkippedTupleStruct>(),
            specta_serde::Format,
        )
        .expect_err("a declared multi-field tuple struct remains a sequence");

    assert!(
        error
            .to_string()
            .contains("tuple structs serialize as a sequence"),
        "the tuple must not be mistaken for its surviving field: {error}"
    );
}

#[test]
fn aliases_inside_flattened_payloads_do_not_exclude_parent_keys() {
    let value = ParentKeyCollidesWithFlattenedAlias {
        outer_key: "outer".into(),
        inner: AliasedFlattenPayload {
            value: "inner".into(),
        },
    };
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::json!({ "outer_key": "outer", "value": "inner" })
    );

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentKeyCollidesWithFlattenedAlias>(),
            specta_serde::Format,
        )
        .expect("flattened payload aliases should not exclude parent keys");
    assert!(
        rendered.contains(
            "export type AliasedFlattenPayload = {\n\tvalue: string,\n\touter_key?: never,\n} | {\n\touter_key: string,\n\tvalue?: never,\n};"
        ),
        "ordinary uses must retain alias exclusions:\n{rendered}"
    );
    let parent = rendered
        .split_once("export type ParentKeyCollidesWithFlattenedAlias = ")
        .expect("parent type should be exported")
        .1;
    assert!(
        !parent.contains("outer_key?: never"),
        "the flattened use must relax alias exclusions:\n{rendered}"
    );
}

#[test]
fn recursive_flattened_aliases_do_not_recurse_during_export() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<RecursiveFlattenedAlias>(),
            specta_serde::Format,
        )
        .expect("recursive flattened aliases should export");

    assert!(
        rendered.contains("export type RecursiveFlattenedAlias"),
        "recursive type should be present:\n{rendered}"
    );
}

#[test]
fn unrelated_flattened_keys_preserve_alias_exclusions() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<AliasWithUnrelatedFlatten>(),
            specta_serde::Format,
        )
        .expect("alias with unrelated flatten should export");

    assert!(
        rendered.contains("x: string,\n\told_x?: never,")
            && rendered.contains("old_x: string,\n\tx?: never,"),
        "ordinary alias exclusions should be preserved:\n{rendered}"
    );
}

#[test]
fn flatten_collisions_relax_only_the_colliding_alias() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<MultipleAliasesWithOneFlattenCollision>(),
            specta_serde::Format,
        )
        .expect("one of multiple aliases may collide with a flattened key");
    let ty = rendered
        .split_once("export type MultipleAliasesWithOneFlattenCollision = ")
        .expect("multi-alias parent should be exported")
        .1;

    assert!(
        !ty.contains("old?: never") && ty.contains("legacy?: never"),
        "only the colliding alias exclusion should be relaxed:\n{rendered}"
    );
}

#[test]
fn skipped_flattened_fields_do_not_relax_alias_exclusions() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<AliasBesideSkippedFlattenedKey>(),
            specta_serde::Format,
        )
        .expect("skipped flattened keys should export");
    let ty = rendered
        .split_once("export type AliasBesideSkippedFlattenedKey = ")
        .expect("skipped-key parent should be exported")
        .1;

    assert!(
        ty.contains("old?: never"),
        "a skipped flattened field cannot supply the alias key:\n{rendered}"
    );
}

#[test]
fn literal_wire_keys_cannot_collide_with_flatten_bookkeeping() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<AliasesBesideFormerSentinelKey>(),
            specta_serde::Format,
        )
        .expect("literal wire keys should remain distinct from bookkeeping");
    let ty = rendered
        .split_once("export type AliasesBesideFormerSentinelKey = ")
        .expect("literal-key parent should be exported")
        .1;

    assert!(
        ty.contains("old?: never") && ty.contains("legacy?: never"),
        "unrelated alias exclusions should remain intact:\n{rendered}"
    );
}

#[test]
fn flattened_maps_relax_alias_exclusions_for_arbitrary_keys() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<AliasWithFlattenedMap>(),
            specta_serde::Format,
        )
        .expect("aliases beside flattened maps should export");
    let ty = rendered
        .split_once("export type AliasWithFlattenedMap = ")
        .expect("map parent should be exported")
        .1;

    assert!(
        !ty.contains("old_value?: never"),
        "the flattened map can provide the alias key:\n{rendered}"
    );
}

#[test]
fn flattened_external_enum_tags_relax_alias_exclusions() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<AliasBesideFlattenedExternalTag>(),
            specta_serde::Format,
        )
        .expect("aliases beside flattened external enums should export");
    let ty = rendered
        .split_once("export type AliasBesideFlattenedExternalTag = ")
        .expect("external-tag parent should be exported")
        .1;

    assert!(
        !ty.contains("Kind?: never"),
        "the flattened enum emits its external tag key:\n{rendered}"
    );
}

#[test]
fn flattened_variant_untagged_payload_keys_relax_alias_exclusions() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<AliasBesideFlattenedVariantUntagged>(),
            specta_serde::Format,
        )
        .expect("aliases beside flattened untagged variants should export");
    let ty = rendered
        .split_once("export type AliasBesideFlattenedVariantUntagged = ")
        .expect("untagged-variant parent should be exported")
        .1;

    assert!(
        !ty.contains("old?: never"),
        "the untagged variant emits its payload key directly:\n{rendered}"
    );
}

#[test]
fn flattened_alias_relaxation_does_not_cross_object_scopes() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<OuterKeyBesideNestedAlias>(),
            specta_serde::Format,
        )
        .expect("nested aliases should export");

    assert!(
        rendered.contains("value: string,\n\touter_key?: never,")
            && rendered.contains("outer_key: string,\n\tvalue?: never,"),
        "nested alias exclusions should remain intact:\n{rendered}"
    );
}

#[test]
fn aliases_through_flattened_generic_wrappers_do_not_exclude_parent_keys() {
    let value = ParentKeyCollidesThroughFlattenedWrapper {
        outer_key: "outer".into(),
        wrapper: FlattenWrapper {
            inner: AliasedFlattenPayload {
                value: "inner".into(),
            },
        },
    };
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::json!({ "outer_key": "outer", "value": "inner" })
    );

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentKeyCollidesThroughFlattenedWrapper>(),
            specta_serde::Format,
        )
        .expect("aliases behind flattened wrappers should export");
    let parent = rendered
        .split_once("export type ParentKeyCollidesThroughFlattenedWrapper = ")
        .expect("parent type should be exported")
        .1;
    assert!(
        !parent.contains("outer_key?: never"),
        "the flattened wrapper use must relax nested alias exclusions:\n{rendered}"
    );
}

#[test]
fn nested_instantiations_of_the_same_flatten_wrapper_are_expanded() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentKeyCollidesThroughDoubleFlattenedWrapper>(),
            specta_serde::Format,
        )
        .expect("nested generic wrapper instantiations should export");
    let parent = rendered
        .split_once("export type ParentKeyCollidesThroughDoubleFlattenedWrapper = ")
        .expect("double-wrapper parent should be exported")
        .1
        .split_once(";\n")
        .expect("parent declaration should terminate")
        .0;

    assert!(
        !parent.contains("outer_key?: never"),
        "the nested generic wrapper must expose its flattened aliases:\n{rendered}"
    );
    assert!(
        !parent.contains("FlattenWrapper"),
        "the contextual nested wrapper must be expanded rather than hidden behind a reference:\n{rendered}"
    );
}

#[test]
fn enum_variant_flattened_references_relax_alias_exclusions() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<EnumVariantWithFlattenedAlias>(),
            specta_serde::Format,
        )
        .expect("enum variant flattened aliases should export");
    let variant = rendered
        .split_once("export type EnumVariantWithFlattenedAlias = ")
        .expect("enum should be exported")
        .1;

    assert!(
        !variant.contains("outer_key?: never"),
        "the contextual enum variant must relax the flattened alias exclusion:\n{rendered}"
    );
}

#[test]
fn parent_aliases_detect_keys_through_flattened_generic_wrappers() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentAliasCollidesThroughFlattenedWrapper>(),
            specta_serde::Format,
        )
        .expect("parent alias collisions through generic wrappers should export");
    let parent = rendered
        .split_once("export type ParentAliasCollidesThroughFlattenedWrapper = ")
        .expect("parent type should be exported")
        .1;

    assert!(
        !parent.contains("value?: never"),
        "the flattened generic key must relax the parent alias exclusion:\n{rendered}"
    );
}

#[test]
fn aliases_through_flattened_conversions_do_not_exclude_parent_keys() {
    let value = ParentKeyCollidesWithConvertedFlattenedAlias {
        outer_key: "outer".into(),
        inner: ConvertedAliasedFlattenPayload {
            value: "inner".into(),
        },
    };
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::json!({ "outer_key": "outer", "value": "inner" })
    );

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentKeyCollidesWithConvertedFlattenedAlias>(),
            specta_serde::Format,
        )
        .expect("aliases behind flattened conversions should export");
    let parent = rendered
        .split_once("export type ParentKeyCollidesWithConvertedFlattenedAlias = ")
        .expect("parent type should be exported")
        .1;
    assert!(
        !parent.contains("outer_key?: never"),
        "the converted flattened use must relax alias exclusions:\n{rendered}"
    );
}

#[test]
fn aliases_through_nested_flattened_conversions_do_not_exclude_parent_keys() {
    let value = ParentKeyCollidesThroughConvertedFlattenedWrapper {
        outer_key: "outer".into(),
        wrapper: FlattenWrapper {
            inner: ConvertedAliasedFlattenPayload {
                value: "inner".into(),
            },
        },
    };
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::json!({ "outer_key": "outer", "value": "inner" })
    );

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentKeyCollidesThroughConvertedFlattenedWrapper>(),
            specta_serde::Format,
        )
        .expect("aliases behind nested flattened conversions should export");
    let parent = rendered
        .split_once("export type ParentKeyCollidesThroughConvertedFlattenedWrapper = ")
        .expect("parent type should be exported")
        .1;
    assert!(
        !parent.contains("outer_key?: never"),
        "the nested converted flattened use must relax alias exclusions:\n{rendered}"
    );
}

#[test]
fn flattened_alias_detection_tracks_sibling_generic_instantiations() {
    let value = ParentKeyCollidesInSecondGenericInstantiation {
        outer_key: "outer".into(),
        duo: DuoFlattenWrapper {
            first: FlattenWrapper {
                inner: PlainFlattenPayload {
                    plain: "plain".into(),
                },
            },
            second: FlattenWrapper {
                inner: AliasedFlattenPayload {
                    value: "inner".into(),
                },
            },
        },
    };
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::json!({
            "outer_key": "outer",
            "plain": "plain",
            "value": "inner",
        })
    );

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ParentKeyCollidesInSecondGenericInstantiation>(),
            specta_serde::Format,
        )
        .expect("each flattened generic instantiation should be inspected");
    let parent = rendered
        .split_once("export type ParentKeyCollidesInSecondGenericInstantiation = ")
        .expect("parent type should be exported")
        .1;
    assert!(
        !parent.contains("outer_key?: never"),
        "the second generic instantiation must retain alias detection:\n{rendered}"
    );
}

#[test]
fn aliases_inside_internal_tag_flattened_payloads_do_not_exclude_sibling_keys() {
    let value = InternalTagWithFlattenedAliasCollision::Value {
        outer_key: "outer".into(),
        inner: AliasedFlattenPayload {
            value: "inner".into(),
        },
    };
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::json!({ "t": "Value", "outer_key": "outer", "value": "inner" })
    );

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<InternalTagWithFlattenedAliasCollision>(),
            specta_serde::Format,
        )
        .expect("internal-tag flattened aliases should export");
    let variant = rendered
        .split_once("export type InternalTagWithFlattenedAliasCollision = ")
        .expect("internal-tag type should be exported")
        .1;
    assert!(
        !variant.contains("outer_key?: never"),
        "the internal-tag flattened use must relax alias exclusions:\n{rendered}"
    );
}

#[test]
fn internal_tag_flattened_payloads_relax_sibling_key_collisions() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<InternalTagWithSiblingFlattenCollision>(),
            specta_serde::Format,
        )
        .expect("sibling flattened alias collisions should export");
    let variant = rendered
        .split_once("export type InternalTagWithSiblingFlattenCollision = ")
        .expect("internal-tag type should be exported")
        .1;

    assert!(
        !variant.contains("x?: never"),
        "a sibling flattened key must relax the alias exclusion:\n{rendered}"
    );
}

#[test]
fn aliases_inside_internal_tag_newtype_payloads_do_not_exclude_tag_keys() {
    let value = InternalTagNewtypeAliasCollision::Value(InternalTagAliasPayload {
        value: "payload".into(),
    });
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::json!({ "t": "Value", "value": "payload" })
    );

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<InternalTagNewtypeAliasCollision>(),
            specta_serde::Format,
        )
        .expect("internal-tag newtype aliases should export");
    assert!(
        rendered
            .contains("export type InternalTagAliasPayload = {\n\tvalue: string,\n\tt?: never,"),
        "ordinary payload uses must retain alias exclusions:\n{rendered}"
    );
    let variant = rendered
        .split_once("export type InternalTagNewtypeAliasCollision = ")
        .expect("internal-tag type should be exported")
        .1;
    assert!(
        !variant.contains("t?: never"),
        "the internally tagged use must relax tag-key exclusions:\n{rendered}"
    );
}

#[test]
fn internal_flatten_opt_exports_union_instead_of_nullable_intersection() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<InternalFlattenOpt>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        !rendered.contains("| null"),
        "flattened Option must not admit bare `null`:\n{rendered}"
    );

    let expected =
        "export type InternalFlattenOpt = {\n\tt: \"A\",\n} & Inner1 | {\n\tt: \"A\",\n};";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[test]
fn flat_two_opt_exports_precedence_correct_union() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FlatTwoOpt>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        !rendered.contains("| null"),
        "flattened Option must not admit bare `null`:\n{rendered}"
    );

    // Four branches covering every independent Some/None combination of the
    // two flattened Options. `&` binds tighter than `|` in TypeScript, so no
    // parens are required around any branch for this to parse correctly as
    // `(Base & Inner1 & Inner2) | (Base & Inner2) | (Base & Inner1) | Base`.
    let expected = "export type FlatTwoOpt = {\n\ta: number,\n} & Inner1 & Inner2 | {\n\ta: number,\n} & Inner2 | {\n\ta: number,\n} & Inner1 | {\n\ta: number,\n};";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[test]
fn flat_default_export_is_unaffected() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FlatDefault>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    let expected = "export type FlatDefault = {\n\ta: number,\n} & DefaultInner;";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[test]
fn flat_nested_opt_exports_same_shape_as_single_option() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FlatNestedOpt>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        !rendered.contains("| null"),
        "flattened Option<Option<T>> must not admit bare `null`:\n{rendered}"
    );

    let expected = "export type FlatNestedOpt = {\n\ta: number,\n} & Inner1 | {\n\ta: number,\n};";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[test]
fn flat_three_opt_exports_deterministic_branch_order() {
    // Pins the exact branch ordering of the 2^k expansion (all-present first,
    // counting the subset bitmask down to the all-absent base) so refactors
    // can't silently introduce nondeterminism - snapshot stability depends on
    // this being reproducible.
    let expected = "export type FlatThreeOpt = {\n\ta: number,\n} & Inner1 & Inner2 & Inner3 | {\n\ta: number,\n} & Inner2 & Inner3 | {\n\ta: number,\n} & Inner1 & Inner3 | {\n\ta: number,\n} & Inner3 | {\n\ta: number,\n} & Inner1 & Inner2 | {\n\ta: number,\n} & Inner2 | {\n\ta: number,\n} & Inner1 | {\n\ta: number,\n};";

    for _ in 0..3 {
        let rendered = Typescript::default()
            .export(
                &Types::default().register::<FlatThreeOpt>(),
                specta_serde::Format,
            )
            .expect("export should succeed");
        assert!(
            rendered.contains(expected),
            "expected:\n{expected}\n\ngot:\n{rendered}"
        );
    }
}

#[test]
fn flat_mixed_exports_mandatory_part_in_every_branch() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FlatMixed>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    // `Req` is a non-Option flatten so it appears in both branches; only the
    // flattened Option toggles.
    let expected =
        "export type FlatMixed = {\n\ta: number,\n} & Req & Inner1 | {\n\ta: number,\n} & Req;";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[test]
fn tag_with_fields_exports_union_with_leftover_fields() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<TagWithFields>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    // The internally tagged variant's regular fields stay in every branch;
    // only the flattened Option toggles.
    let expected = "export type TagWithFields = {\n\tt: \"A\",\n} & {\n\tb: boolean,\n} & Inner1 | {\n\tt: \"A\",\n} & {\n\tb: boolean,\n};";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[test]
fn flat_opt_map_exports_union_with_index_signature() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FlatOptMap>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        !rendered.contains("| null"),
        "flattened Option must not admit bare `null`:\n{rendered}"
    );

    let expected = "export type FlatOptMap = {\n\ta: number,\n} & { [key in string]: number } | {\n\ta: number,\n};";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[test]
fn flat_opt_phases_format_does_not_split() {
    // A flattened Option on its own is phase-symmetric: serialization and
    // deserialization see the same union, so `PhasesFormat` must not emit
    // separate `_Serialize`/`_Deserialize` types for it.
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FlatOpt>(),
            specta_serde::PhasesFormat,
        )
        .expect("export should succeed");

    assert!(
        !rendered.contains("_Serialize") && !rendered.contains("_Deserialize"),
        "flattened Option alone must not trigger a phase split:\n{rendered}"
    );

    let expected = "export type FlatOpt = {\n\ta: number,\n} & Inner1 | {\n\ta: number,\n};";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}
