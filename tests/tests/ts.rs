use std::{
    cell::RefCell,
    collections::HashMap,
    convert::Infallible,
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    path::PathBuf,
};

use serde::Serialize;
use specta::Type;
use specta_typescript::{BigIntExportBehavior, ExportError, ExportPath, NamedLocation, Typescript};
use specta_util::Any;

macro_rules! assert_ts {
    (error; $t:ty, $e:expr) => {
        assert_eq!(
            specta_typescript::inline::<$t>(&Default::default()),
            Err($e.into())
        )
    };
    ($t:ty, $e:expr) => {
        assert_eq!(
            specta_typescript::inline::<$t>(&Default::default()),
            Ok($e.into())
        )
    };

    (() => $expr:expr, $e:expr) => {
        let _: () = {
            fn assert_ty_eq<T: Type>(_t: T) {
                assert_eq!(
                    specta_typescript::inline::<T>(&Default::default()),
                    Ok($e.into())
                );
            }
            assert_ty_eq($expr);
        };
    };
}
pub(crate) use assert_ts;

macro_rules! assert_ts_export {
    ($t:ty, $e:expr) => {
        assert_eq!(
            specta_typescript::export::<$t>(&Default::default()),
            Ok($e.into())
        )
    };
    (error; $t:ty, $e:expr) => {
        assert_eq!(
            specta_typescript::export::<$t>(&Default::default()),
            Err($e.into())
        )
    };
    ($t:ty, $e:expr; $cfg:expr) => {
        assert_eq!(specta_typescript::export::<$t>($cfg), Ok($e.into()))
    };
    (error; $t:ty, $e:expr; $cfg:expr) => {
        assert_eq!(specta_typescript::export::<$t>($cfg), Err($e.into()))
    };
}
pub(crate) use assert_ts_export;

// TODO: Unit test other `specta::Type` methods such as `::reference(...)`

#[test]
fn typescript_types() {
    assert_ts!(Vec<MyEnum>, r#"({ A: string } | { B: number })[]"#);

    assert_ts!(i8, "number");
    assert_ts!(u8, "number");
    assert_ts!(i16, "number");
    assert_ts!(u16, "number");
    assert_ts!(i32, "number");
    assert_ts!(u32, "number");
    assert_ts!(f32, "number");
    assert_ts!(f64, "number");

    assert_ts!(bool, "boolean");

    assert_ts!((), "null");
    assert_ts!((String, i32), "[string, number]");
    assert_ts!((String, i32, bool), "[string, number, boolean]");
    assert_ts!(
        (bool, bool, bool, bool, bool, bool, bool, bool, bool, bool, bool, bool),
        "[boolean, boolean, boolean, boolean, boolean, boolean, boolean, boolean, boolean, boolean, boolean, boolean]"
    );

    assert_ts!(String, "string");
    // impossible since Path as a generic is unsized lol
    // assert_ts!(Path, "string");
    assert_ts!(PathBuf, "string");
    assert_ts!(IpAddr, "string");
    assert_ts!(Ipv4Addr, "string");
    assert_ts!(Ipv6Addr, "string");
    assert_ts!(SocketAddr, "string");
    assert_ts!(SocketAddrV4, "string");
    assert_ts!(SocketAddrV6, "string");
    assert_ts!(char, "string");
    assert_ts!(&'static str, "string");

    assert_ts!(&'static bool, "boolean");
    assert_ts!(&'static i32, "number");

    assert_ts!(Vec<i32>, "number[]");
    assert_ts!(&[i32], "number[]");
    assert_ts!(&[i32; 3], "[number, number, number]");

    assert_ts!(Option<i32>, "number | null");

    // https://github.com/oscartbeaumont/specta/issues/88
    assert_ts!(Unit1, "null");
    assert_ts!(Unit2, "Record<string, never>");
    assert_ts!(Unit3, "[]");
    assert_ts!(Unit4, "null");
    assert_ts!(Unit5, r#""A""#);
    assert_ts!(Unit6, "{ A: [] }");
    assert_ts!(Unit7, "{ A: Record<string, never> }");

    assert_ts!(
        SimpleStruct,
        "{ a: number; b: string; c: [number, string, number]; d: string[]; e: string | null }"
    );
    assert_ts!(TupleStruct1, "number");
    assert_ts!(TupleStruct3, "[number, boolean, string]");

    assert_ts!(
        TestEnum,
        r#""Unit" | { Single: number } | { Multiple: [number, number] } | { Struct: { a: number } }"#
    );
    assert_ts!(RefStruct, "TestEnum");

    assert_ts!(
        InlinerStruct,
        "{ inline_this: { ref_struct: SimpleStruct; val: number }; dont_inline_this: RefStruct }"
    );

    assert_ts!(GenericStruct<i32>, "{ arg: number }");
    assert_ts!(GenericStruct<String>, "{ arg: string }");

    assert_ts!(
        FlattenEnumStruct,
        r#"({ tag: "One" } | { tag: "Two" } | { tag: "Three" }) & { outer: string }"#
    );

    assert_ts!(OverridenStruct, "{ overriden_field: string }");
    assert_ts!(HasGenericAlias, r#"Partial<{ [key in number]: string }>"#);

    assert_ts!(SkipVariant, "{ A: string }");
    assert_ts!(SkipVariant2, r#"{ tag: "A"; data: string }"#);
    assert_ts!(SkipVariant3, "{ A: { a: string } }");

    assert_ts!(
        EnumMacroAttributes,
        "{ A: string } | { bbb: number } | { cccc: number } | { D: { a: string; bbbbbb: number } }"
    );

    assert_ts!(Recursive, "{ a: number; children: Recursive[] }");

    assert_ts!(InlineEnumField, "{ A: { a: string } }");

    assert_ts!(
        InlineOptionalType,
        "{ optional_field: PlaceholderInnerField | null }"
    );

    assert_ts_export!(
        RenameToValue,
        "export type RenameToValueNewName = { demo_new_name: number }"
    );

    assert_ts!(Rename, r#""OneWord" | "Two words""#);

    assert_ts!(TransparentType, r#"TransparentTypeInner"#); // TODO: I don't think this is correct for `Type::inline`
    assert_ts!(TransparentType2, r#"null"#);
    assert_ts!(TransparentTypeWithOverride, r#"string"#);

    // I love serde but this is so mega cringe. Lack of support and the fact that `0..5` == `0..=5` is so dumb.
    assert_ts!(() => 0..5, r#"{ start: number; end: number }"#);
    // assert_ts!(() => 0.., r#"{ start: 0 }"#);
    // assert_ts!(() => .., r#""#);
    assert_ts!(() => 0..=5, r#"{ start: number; end: number }"#);
    // assert_ts!(() => ..5, r#"{ end: 5 }"#);
    // assert_ts!(() => ..=5, r#"{ end: 5 }"#);

    // https://github.com/oscartbeaumont/specta/issues/66
    assert_ts!(
        [Option<u8>; 3],
        r#"[(number | null), (number | null), (number | null)]"#
    );

    // https://github.com/oscartbeaumont/specta/issues/65
    assert_ts!(HashMap<BasicEnum, ()>, r#"Partial<{ [key in "A" | "B"]: null }>"#);

    // https://github.com/oscartbeaumont/specta/issues/60
    assert_ts!(Option<Option<Option<Option<i32>>>>, r#"number | null"#);

    // https://github.com/oscartbeaumont/specta/issues/71
    assert_ts!(Vec<PlaceholderInnerField>, r#"{ a: string }[]"#);

    // https://github.com/oscartbeaumont/specta/issues/77
    assert_eq!(
        specta_typescript::inline::<std::time::SystemTime>(
            &Typescript::new().bigint(BigIntExportBehavior::Number)
        ),
        Ok(r#"{ duration_since_epoch: number; duration_since_unix_epoch: number }"#.into())
    );
    assert_eq!(
        specta_typescript::inline::<std::time::SystemTime>(
            &Typescript::new().bigint(BigIntExportBehavior::String)
        ),
        Ok(r#"{ duration_since_epoch: string; duration_since_unix_epoch: number }"#.into())
    );

    assert_eq!(
        specta_typescript::inline::<std::time::Duration>(
            &Typescript::new().bigint(BigIntExportBehavior::Number)
        ),
        Ok(r#"{ secs: number; nanos: number }"#.into())
    );
    assert_eq!(
        specta_typescript::inline::<std::time::Duration>(
            &Typescript::new().bigint(BigIntExportBehavior::String)
        ),
        Ok(r#"{ secs: string; nanos: number }"#.into())
    );

    assert_ts!(HashMap<BasicEnum, i32>, r#"Partial<{ [key in "A" | "B"]: number }>"#);
    assert_ts_export!(
        EnumReferenceRecordKey,
        "export type EnumReferenceRecordKey = { a: Partial<{ [key in BasicEnum]: number }> }"
    );

    assert_ts!(
        FlattenOnNestedEnum,
        r#"({ type: "a"; value: string } | { type: "b"; value: number }) & { id: string }"#
    );

    assert_ts!(PhantomData<()>, r#"null"#);
    assert_ts!(PhantomData<String>, r#"null"#);
    assert_ts!(Infallible, r#"never"#);

    // assert_ts!(Result<String, i32>, r#"string | number"#);
    // assert_ts!(Result<i16, i32>, r#"number"#);

    #[cfg(feature = "either")]
    {
        assert_ts!(either::Either<String, i32>, r#"string | number"#);
        assert_ts!(either::Either<i16, i32>, r#"number"#);
    }

    assert_ts!(Any, r#"any"#);

    assert_ts!(MyEmptyInput, "Record<string, never>");
    assert_ts_export!(
        MyEmptyInput,
        "export type MyEmptyInput = Record<string, never>"
    );

    // https://github.com/oscartbeaumont/specta/issues/142
    #[allow(unused_parens)]
    {
        assert_ts!((String), r#"string"#);
        assert_ts!((String,), r#"[string]"#);
    }

    // https://github.com/oscartbeaumont/specta/issues/148
    assert_ts!(ExtraBracketsInTupleVariant, "{ A: string }");
    assert_ts!(ExtraBracketsInUnnamedStruct, "string");

    // https://github.com/oscartbeaumont/specta/issues/156
    assert_ts!(Vec<MyEnum>, r#"({ A: string } | { B: number })[]"#);

    assert_ts!(InlineTuple, r#"{ demo: [string, boolean] }"#);
    assert_ts!(
        InlineTuple2,
        r#"{ demo: [{ demo: [string, boolean] }, boolean] }"#
    );

    // https://github.com/oscartbeaumont/specta/issues/220
    assert_ts!(Box<str>, r#"string"#);

    assert_ts!(
        SkippedFieldWithinVariant,
        r#"{ type: "A" } | { type: "B"; data: string }"#
    );

    // https://github.com/oscartbeaumont/specta/issues/239
    assert_ts!(KebabCase, r#"{ "test-ing": string }"#);

    // https://github.com/specta-rs/specta/issues/281
    assert_ts!(&[&str], "string[]");
    assert_ts!(Issue281<'_>, "{ default_unity_arguments: string[] }");

    // https://github.com/oscartbeaumont/specta/issues/90
    assert_ts!(RenameWithWeirdCharsField, r#"{ "@odata.context": string }"#);
    assert_ts!(
        RenameWithWeirdCharsVariant,
        r#"{ "@odata.context": string }"#
    );
    // TODO: Reenable these tests when they are no so flaky
    // assert_ts_export!(
    //     error;
    //     RenameWithWeirdCharsStruct,
    //     ExportError::InvalidName(
    //         NamedLocation::Type,
    //         #[cfg(not(windows))]
    //         ExportPath::new_unsafe("tests/tests/ts.rs:640:10"),
    //         #[cfg(windows)]
    //         ExportPath::new_unsafe("tests\tests\ts.rs:640:10"),
    //         r#"@odata.context"#.to_string()
    //     )
    // );
    // assert_ts_export!(
    //     error;
    //     RenameWithWeirdCharsEnum,
    //     ExportError::InvalidName(
    //         NamedLocation::Type,
    //         #[cfg(not(windows))]
    //         ExportPath::new_unsafe("tests/tests/ts.rs:644:10"),
    //         #[cfg(windows)]
    //         ExportPath::new_unsafe("tests\tests\ts.rs:644:10"),
    //         r#"@odata.context"#.to_string()
    //     )
    // );
}

#[derive(Type)]
#[specta(export = false)]
struct Unit1;

#[derive(Type)]
#[specta(export = false)]
struct Unit2 {}

#[derive(Type)]
#[specta(export = false)]
struct Unit3();

#[derive(Type)]
#[specta(export = false)]
struct Unit4(());

#[derive(Type)]
#[specta(export = false)]
enum Unit5 {
    A,
}

#[derive(Type)]
#[specta(export = false)]
enum Unit6 {
    A(),
}

#[derive(Type)]
#[specta(export = false)]
enum Unit7 {
    A {},
}

#[derive(Type)]
#[specta(export = false)]
struct SimpleStruct {
    a: i32,
    b: String,
    c: (i32, String, RefCell<i32>),
    d: Vec<String>,
    e: Option<String>,
}

#[derive(Type)]
#[specta(export = false)]
struct TupleStruct1(i32);

#[derive(Type)]
#[specta(export = false)]
struct TupleStruct3(i32, bool, String);

#[derive(Type)]
#[specta(export = false)]
#[specta(rename = "HasBeenRenamed")]
struct RenamedStruct;

#[derive(Type)]
#[specta(export = false)]
enum TestEnum {
    Unit,
    Single(i32),
    Multiple(i32, i32),
    Struct { a: i32 },
}

#[derive(Type)]
#[specta(export = false)]
struct RefStruct(TestEnum);

#[derive(Type)]
#[specta(export = false)]
struct InlineStruct {
    ref_struct: SimpleStruct,
    val: i32,
}

#[derive(Type)]
#[specta(export = false)]
struct InlinerStruct {
    #[specta(inline)]
    inline_this: InlineStruct,
    dont_inline_this: RefStruct,
}

#[derive(Type)]
#[specta(export = false)]
struct GenericStruct<T> {
    arg: T,
}

#[derive(Serialize, Type)]
#[specta(export = false)]
struct FlattenEnumStruct {
    outer: String,
    #[serde(flatten)]
    inner: FlattenEnum,
}

#[derive(Serialize, Type)]
#[specta(export = false)]
#[serde(tag = "tag", content = "test")]
enum FlattenEnum {
    One,
    Two,
    Three,
}

#[derive(Serialize, Type)]
#[specta(export = false)]
struct OverridenStruct {
    #[specta(type = String)]
    overriden_field: i32,
}

#[derive(Type)]
#[specta(export = false)]
struct HasGenericAlias(GenericAlias<i32>);

type GenericAlias<T> = std::collections::HashMap<T, String>;

#[derive(Serialize, Type)]
#[specta(export = false)]
enum SkipVariant {
    A(String),
    #[serde(skip)]
    B(i32),
    #[specta(skip)]
    C(i32),
}

#[derive(Serialize, Type)]
#[specta(export = false)]
#[serde(tag = "tag", content = "data")]
enum SkipVariant2 {
    A(String),
    #[serde(skip)]
    B(i32),
    #[specta(skip)]
    C(i32),
}

#[derive(Serialize, Type)]
#[specta(export = false)]
enum SkipVariant3 {
    A {
        a: String,
    },
    #[serde(skip)]
    B {
        b: i32,
    },
    #[specta(skip)]
    C {
        b: i32,
    },
}

#[derive(Type)]
#[specta(export = false)]
pub enum EnumMacroAttributes {
    A(#[specta(type = String)] i32),
    #[specta(rename = "bbb")]
    B(i32),
    #[specta(rename = "cccc")]
    C(#[specta(type = i32)] String),
    D {
        #[specta(type = String)]
        a: i32,
        #[specta(rename = "bbbbbb")]
        b: i32,
    },
}

#[derive(Type)]
#[specta(export = false)]
pub struct PlaceholderInnerField {
    a: String,
}

#[derive(Type)]
#[specta(export = false)]
pub struct Recursive {
    a: i32,
    children: Vec<Recursive>,
}

#[derive(Type)]
#[specta(export = false)]

pub enum InlineEnumField {
    #[specta(inline)]
    A(PlaceholderInnerField),
}

#[derive(Type)]
#[specta(export = false)]
pub struct InlineOptionalType {
    #[specta(inline)]
    pub optional_field: Option<PlaceholderInnerField>,
}

const CONTAINER_NAME: &str = "RenameToValueNewName";
const FIELD_NAME: &str = "demo_new_name";

// This is very much an advanced API. It is not recommended to use this unless you know what your doing.
// For personal reference: Is used in PCR to apply an inflection to the dynamic name of the include/select macro.
#[derive(Type)]
#[specta(export = false, rename_from_path = CONTAINER_NAME)]
pub struct RenameToValue {
    #[specta(rename_from_path = FIELD_NAME)]
    pub demo: i32,
}

// Regression test for https://github.com/oscartbeaumont/specta/issues/56
#[derive(Type, serde::Serialize)]
#[specta(export = false)]
enum Rename {
    OneWord,
    #[serde(rename = "Two words")]
    TwoWords,
}

#[derive(Type, serde::Serialize)]
#[specta(export = false)]
pub struct TransparentTypeInner {
    inner: String,
}

#[derive(Type, serde::Serialize)]
#[specta(export = false)]
#[serde(transparent)]
pub struct TransparentType(pub(crate) TransparentTypeInner);

#[derive(Type, serde::Serialize)]
#[specta(export = false)]
#[serde(transparent)]
pub struct TransparentType2(pub(crate) ());

#[derive(serde::Serialize)]
pub struct NonTypeType;

#[derive(Type, serde::Serialize)]
#[specta(export = false)]
#[serde(transparent)]
pub struct TransparentTypeWithOverride(#[specta(type = String)] NonTypeType);

#[derive(Type, serde::Serialize)]
#[specta(export = false)]
pub enum BasicEnum {
    A,
    B,
}

#[derive(Type)]
#[serde(
    export = false,
    tag = "type",
    content = "value",
    rename_all = "camelCase"
)]
pub enum NestedEnum {
    A(String),
    B(i32),
}

#[derive(Type)]
#[serde(export = false, rename_all = "camelCase")]
pub struct FlattenOnNestedEnum {
    id: String,
    #[serde(flatten)]
    result: NestedEnum,
}

#[derive(Type)]
#[specta(export = false)]
pub struct EnumReferenceRecordKey {
    a: HashMap<BasicEnum, i32>,
}

// https://github.com/oscartbeaumont/specta/issues/88
#[derive(Type)]
#[serde(export = false, rename_all = "camelCase")]
#[serde(default)]
pub(super) struct MyEmptyInput {}

#[derive(Type)]
#[specta(export = false)]
#[allow(unused_parens)]
pub enum ExtraBracketsInTupleVariant {
    A((String)),
}

#[derive(Type)]
#[specta(export = false)]
#[allow(unused_parens)]
pub struct ExtraBracketsInUnnamedStruct((String));

#[derive(Type)]
#[specta(export = false)]
#[allow(unused_parens)]
pub struct RenameWithWeirdCharsField {
    #[specta(rename = "@odata.context")]
    odata_context: String,
}

#[derive(Type)]
#[specta(export = false)]
#[allow(unused_parens)]
pub enum RenameWithWeirdCharsVariant {
    #[specta(rename = "@odata.context")]
    A(String),
}

#[derive(Type)]
#[specta(export = false, rename = "@odata.context")]
pub struct RenameWithWeirdCharsStruct(String);

#[derive(Type)]
#[specta(export = false, rename = "@odata.context")]
pub enum RenameWithWeirdCharsEnum {}

#[derive(Type)]
pub enum MyEnum {
    A(String),
    B(u32),
}

#[derive(Type)]
pub struct InlineTuple {
    #[specta(inline)]
    demo: (String, bool),
}

#[derive(Type)]
pub struct InlineTuple2 {
    #[specta(inline)]
    demo: (InlineTuple, bool),
}

#[derive(Type)]
#[serde(tag = "type", content = "data")]
pub enum SkippedFieldWithinVariant {
    A(#[serde(skip)] String),
    B(String),
}

#[derive(Type)]
#[serde(rename_all = "kebab-case")]
pub struct KebabCase {
    test_ing: String,
}

// https://github.com/specta-rs/specta/issues/281
#[derive(Type)]
pub struct Issue281<'a> {
    default_unity_arguments: &'a [&'a str],
}
