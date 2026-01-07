use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    convert::Infallible,
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    panic::Location,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use specta::{Type, TypeCollection, datatype::DataType};
use specta_typescript::Any;
use specta_typescript::{BigIntExportBehavior, Typescript};

// We run tests with the low-level APIs
#[track_caller]
pub fn assert_ts_export2<T: Type>() -> Result<String, String> {
    let mut types = TypeCollection::default();
    let ndt = match T::definition(&mut types) {
        DataType::Reference(r) => r.get(&types).expect("Can't find type in `TypeCollection`"),
        _ => panic!("This type can't be exported!"),
    };

    specta_typescript::primitives::export(
        &Typescript::default().bigint(BigIntExportBehavior::Number),
        &types,
        ndt,
    )
    .map_err(|e| e.to_string())
}
pub fn assert_ts_inline2<T: Type>() -> Result<String, String> {
    let mut types = TypeCollection::default();
    let dt = T::definition(&mut types);
    specta_typescript::primitives::inline(
        &Typescript::default().bigint(BigIntExportBehavior::Number),
        &types,
        &dt,
    )
    .map_err(|e| e.to_string())
}

macro_rules! assert_ts {
    (error; $t:ty, $e:expr) => {
        assert_eq!(
            crate::ts::inline::<$t>(&Default::default()),
            Err($e.to_string())
        )
    };
    ($t:ty, $e:expr) => {
        assert_eq!(crate::ts::inline::<$t>(&Default::default()), Ok($e.into()))
    };

    (() => $expr:expr, $e:expr) => {
        let _: () = {
            fn assert_ty_eq<T: specta::Type>(_t: T) {
                assert_eq!(crate::ts::inline::<T>(&Default::default()), Ok($e.into()));
            }
            assert_ty_eq($expr);
        };
    };
}
pub(crate) use assert_ts;

macro_rules! assert_ts_export {
    ($t:ty, $e:expr) => {
        assert_eq!(
            crate::ts::export::<$t>(&Default::default()).map_err(|e| e.to_string()),
            Ok($e.into())
        )
    };
    (error; $t:ty, $e:expr) => {
        assert_eq!(
            crate::ts::export::<$t>(&Default::default()).map_err(|e| e.to_string()),
            Err($e.to_string())
        )
    };
    ($t:ty, $e:expr; $cfg:expr) => {
        assert_eq!(crate::ts::export::<$t>($cfg), Ok($e.into()))
    };
    (error; $t:ty, $e:expr; $cfg:expr) => {
        assert_eq!(crate::ts::export::<$t>($cfg), Err($e.into()))
    };
}
pub(crate) use assert_ts_export;

pub fn inline_ref<T: Type>(t: &T, ts: &Typescript) -> Result<String, String> {
    inline::<T>(ts)
}

// TODO: Probally move to snapshot testing w/ high-level API's
pub fn inline<T: Type>(ts: &Typescript) -> Result<String, String> {
    let mut types = TypeCollection::default();
    let dt = T::definition(&mut types);

    // TODO: Could we remove this? It's for backwards compatibility.
    {
        if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&types).into_iter().next() {
            return Err(
                specta_typescript::Error::DuplicateTypeNameLegacy(ty_name, l0, l1).to_string(),
            );
        }
    }

    specta_typescript::primitives::inline(ts, &types, &dt)
        // Allows matching the value. Implementing `PartialEq` on it is really hard.
        .map_err(|e| e.to_string())
}

pub fn export_ref<T: Type>(t: &T, ts: &Typescript) -> Result<String, String> {
    export::<T>(ts)
}

// TODO: Probally move to snapshot testing w/ high-level API's
pub fn export<T: Type>(ts: &Typescript) -> Result<String, String> {
    let mut types = TypeCollection::default();
    T::definition(&mut types);

    // TODO: Could we remove this? It's for backwards compatibility.
    {
        if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&types).into_iter().next() {
            return Err(
                specta_typescript::Error::DuplicateTypeNameLegacy(ty_name, l0, l1).to_string(),
            );
        }
    }

    // let mut ndt = types.get(T::ID).unwrap().clone();
    // specta_typescript::legacy::inline_and_flatten_ndt(&mut ndt, &types);
    // specta_typescript::primitives::export(ts, &types, &ndt)
    // Allows matching the value. Implementing `PartialEq` on it is really hard.
    // .map_err(|e| e.to_string())
    todo!();
}

fn detect_duplicate_type_names(
    types: &TypeCollection,
) -> Vec<(Cow<'static, str>, Location<'static>, Location<'static>)> {
    let mut errors = Vec::new();

    // let mut map = HashMap::with_capacity(types.len());
    // for dt in types.into_unsorted_iter() {
    //     if let Some((existing_sid, existing_impl_location)) =
    //         map.insert(dt.name().clone(), (dt.sid(), dt.location()))
    //     {
    //         if existing_sid != dt.sid() {
    //             errors.push((dt.name().clone(), dt.location(), existing_impl_location));
    //         }
    //     }
    // }
    todo!();

    errors
}

// TODO: Unit test other `specta::Type` methods such as `::reference(...)`

#[test]
fn typescript_types() {
    insta::assert_snapshot!(inline::<Vec<MyEnum>>(&Default::default()).unwrap(), @r#"({ A: string } | { B: number })[]"#);

    insta::assert_snapshot!(inline::<i8>(&Default::default()).unwrap(), @"number");
    insta::assert_snapshot!(inline::<u8>(&Default::default()).unwrap(), @"number");
    insta::assert_snapshot!(inline::<i16>(&Default::default()).unwrap(), @"number");
    insta::assert_snapshot!(inline::<u16>(&Default::default()).unwrap(), @"number");
    insta::assert_snapshot!(inline::<i32>(&Default::default()).unwrap(), @"number");
    insta::assert_snapshot!(inline::<u32>(&Default::default()).unwrap(), @"number");
    insta::assert_snapshot!(inline::<f32>(&Default::default()).unwrap(), @"number");
    insta::assert_snapshot!(inline::<f64>(&Default::default()).unwrap(), @"number");

    insta::assert_snapshot!(inline::<bool>(&Default::default()).unwrap(), @"boolean");

    insta::assert_snapshot!(inline::<()>(&Default::default()).unwrap(), @"null");
    insta::assert_snapshot!(inline::<(String, i32)>(&Default::default()).unwrap(), @"[string, number]");
    insta::assert_snapshot!(inline::<(String, i32, bool)>(&Default::default()).unwrap(), @"[string, number, boolean]");
    insta::assert_snapshot!(
        inline::<(
            bool, bool, bool, bool, bool, bool, bool, bool, bool, bool, bool, bool
        )>(&Default::default()).unwrap(),
        @"[boolean, boolean, boolean, boolean, boolean, boolean, boolean, boolean, boolean, boolean, boolean, boolean]"
    );

    insta::assert_snapshot!(inline::<String>(&Default::default()).unwrap(), @"string");
    // impossible since Path as a generic is unsized lol
    // insta::assert_snapshot!(inline::<Path>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(inline::<PathBuf>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(inline::<IpAddr>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(inline::<Ipv4Addr>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(inline::<Ipv6Addr>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(inline::<SocketAddr>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(inline::<SocketAddrV4>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(inline::<SocketAddrV6>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(inline::<char>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(inline::<&'static str>(&Default::default()).unwrap(), @"string");

    insta::assert_snapshot!(inline::<&'static bool>(&Default::default()).unwrap(), @"boolean");
    insta::assert_snapshot!(inline::<&'static i32>(&Default::default()).unwrap(), @"number");

    insta::assert_snapshot!(inline::<Vec<i32>>(&Default::default()).unwrap(), @"number[]");
    insta::assert_snapshot!(inline::<&[i32]>(&Default::default()).unwrap(), @"number[]");
    insta::assert_snapshot!(inline::<&[i32; 3]>(&Default::default()).unwrap(), @"[number, number, number]");

    insta::assert_snapshot!(inline::<Option<i32>>(&Default::default()).unwrap(), @"number | null");

    // https://github.com/oscartbeaumont/specta/issues/88
    insta::assert_snapshot!(inline::<Unit1>(&Default::default()).unwrap(), @"null");
    insta::assert_snapshot!(inline::<Unit2>(&Default::default()).unwrap(), @"Record<string, never>");
    insta::assert_snapshot!(inline::<Unit3>(&Default::default()).unwrap(), @"[]");
    insta::assert_snapshot!(inline::<Unit4>(&Default::default()).unwrap(), @"null");
    insta::assert_snapshot!(inline::<Unit5>(&Default::default()).unwrap(), @r#""A""#);
    insta::assert_snapshot!(inline::<Unit6>(&Default::default()).unwrap(), @"{ A: [] }");
    insta::assert_snapshot!(inline::<Unit7>(&Default::default()).unwrap(), @"{ A: Record<string, never> }");

    insta::assert_snapshot!(
        inline::<SimpleStruct>(&Default::default()).unwrap(),
        @"{ a: number; b: string; c: [number, string, number]; d: string[]; e: string | null }"
    );
    insta::assert_snapshot!(inline::<TupleStruct1>(&Default::default()).unwrap(), @"number");
    insta::assert_snapshot!(inline::<TupleStruct3>(&Default::default()).unwrap(), @"[number, boolean, string]");

    insta::assert_snapshot!(
        inline::<TestEnum>(&Default::default()).unwrap(),
        @r#""Unit" | { Single: number } | { Multiple: [number, number] } | { Struct: { a: number } }"#
    );
    insta::assert_snapshot!(inline::<RefStruct>(&Default::default()).unwrap(), @"TestEnum");

    insta::assert_snapshot!(
        inline::<InlinerStruct>(&Default::default()).unwrap(),
        @"{ inline_this: { ref_struct: SimpleStruct; val: number }; dont_inline_this: RefStruct }"
    );

    insta::assert_snapshot!(inline::<GenericStruct<i32>>(&Default::default()).unwrap(), @"{ arg: number }");
    insta::assert_snapshot!(inline::<GenericStruct<String>>(&Default::default()).unwrap(), @"{ arg: string }");

    insta::assert_snapshot!(inline::<FlattenEnumStruct>(&Default::default()).unwrap(), @r#"(FlattenEnum) & { outer: string }"#);

    insta::assert_snapshot!(inline::<OverridenStruct>(&Default::default()).unwrap(), @"{ overriden_field: string }");
    insta::assert_snapshot!(inline::<HasGenericAlias>(&Default::default()).unwrap(), @r#"{ [key in number]: string }"#);

    insta::assert_snapshot!(inline::<SkipVariant>(&Default::default()).unwrap(), @"{ A: string }");
    insta::assert_snapshot!(inline::<SkipVariant2>(&Default::default()).unwrap(), @r#"{ tag: "A"; data: string }"#);
    insta::assert_snapshot!(inline::<SkipVariant3>(&Default::default()).unwrap(), @"{ A: { a: string } }");

    insta::assert_snapshot!(
        inline::<EnumMacroAttributes>(&Default::default()).unwrap(),
        @"{ A: string } | { bbb: number } | { cccc: number } | { D: { a: string; bbbbbb: number } }"
    );

    insta::assert_snapshot!(inline::<Recursive>(&Default::default()).unwrap(), @"{ a: number; children: Recursive[] }"); // TODO: FIX

    insta::assert_snapshot!(inline::<InlineEnumField>(&Default::default()).unwrap(), @"{ A: { a: string } }");

    insta::assert_snapshot!(
        inline::<InlineOptionalType>(&Default::default()).unwrap(),
        @"{ optional_field: { a: string } | null }"
    );

    insta::assert_snapshot!(inline::<Rename>(&Default::default()).unwrap(), @r#""OneWord" | "Two words""#);

    insta::assert_snapshot!(inline::<TransparentType>(&Default::default()).unwrap(), @r#"{ inner: string }"#);
    insta::assert_snapshot!(inline::<TransparentType2>(&Default::default()).unwrap(), @r#"null"#);
    insta::assert_snapshot!(inline::<TransparentTypeWithOverride>(&Default::default()).unwrap(), @r#"string"#);

    // I love serde but this is so mega cringe. Lack of support and the fact that `0..5` == `0..=5` is so dumb.
    insta::assert_snapshot!(inline_ref(&(0..5), &Default::default()).unwrap(), @r#"{ start: number; end: number }"#);
    // insta::assert_snapshot!(inline_ref(&(0..), &Default::default()).unwrap(), @r#"{ start: 0 }"#);
    // insta::assert_snapshot!(inline_ref(&(..), &Default::default()).unwrap(), @r#""#);
    insta::assert_snapshot!(inline_ref(&(0..=5), &Default::default()).unwrap(), @r#"{ start: number; end: number }"#);
    // insta::assert_snapshot!(inline_ref(&(..5), &Default::default()).unwrap(), @r#"{ end: 5 }"#);
    // insta::assert_snapshot!(inline_ref(&(..=5), &Default::default()).unwrap(), @r#"{ end: 5 }"#);

    // https://github.com/oscartbeaumont/specta/issues/66
    insta::assert_snapshot!(
        inline::<[Option<u8>; 3]>(&Default::default()).unwrap(),
        @r#"[(number | null), (number | null), (number | null)]"#
    );

    // https://github.com/oscartbeaumont/specta/issues/65
    insta::assert_snapshot!(inline::<HashMap<BasicEnum, ()>>(&Default::default()).unwrap(), @r#"Partial<{ [key in "A" | "B"]: null }>"#);

    // https://github.com/oscartbeaumont/specta/issues/60
    insta::assert_snapshot!(inline::<Option<Option<Option<Option<i32>>>>>(&Default::default()).unwrap(), @r#"number | null"#);

    // https://github.com/oscartbeaumont/specta/issues/71
    insta::assert_snapshot!(inline::<Vec<PlaceholderInnerField>>(&Default::default()).unwrap(), @r#"{ a: string }[]"#);

    // https://github.com/oscartbeaumont/specta/issues/77
    insta::assert_snapshot!(
        inline::<std::time::SystemTime>(&Typescript::new().bigint(BigIntExportBehavior::Number))
            .unwrap(),
        @r#"{ duration_since_epoch: number; duration_since_unix_epoch: number }"#
    );
    insta::assert_snapshot!(
        inline::<std::time::SystemTime>(&Typescript::new().bigint(BigIntExportBehavior::String))
            .unwrap(),
        @r#"{ duration_since_epoch: string; duration_since_unix_epoch: number }"#
    );

    insta::assert_snapshot!(
        inline::<std::time::Duration>(&Typescript::new().bigint(BigIntExportBehavior::Number))
            .unwrap(),
        @r#"{ secs: number; nanos: number }"#
    );
    insta::assert_snapshot!(
        inline::<std::time::Duration>(&Typescript::new().bigint(BigIntExportBehavior::String))
            .unwrap(),
        @r#"{ secs: string; nanos: number }"#
    );

    insta::assert_snapshot!(inline::<HashMap<BasicEnum, i32>>(&Default::default()).unwrap(), @r#"Partial<{ [key in "A" | "B"]: number }>"#);
    insta::assert_snapshot!(
        export::<EnumReferenceRecordKey>(&Default::default()).unwrap(),
        @"export type EnumReferenceRecordKey = { a: Partial<{ [key in BasicEnum]: number }> };"
    );

    insta::assert_snapshot!(inline::<FlattenOnNestedEnum>(&Default::default()).unwrap(), @r#"(NestedEnum) & { id: string }"#);

    insta::assert_snapshot!(inline::<PhantomData<()>>(&Default::default()).unwrap(), @r#"null"#);
    insta::assert_snapshot!(inline::<PhantomData<String>>(&Default::default()).unwrap(), @r#"null"#);
    insta::assert_snapshot!(inline::<Infallible>(&Default::default()).unwrap(), @r#"never"#);

    insta::assert_snapshot!(inline::<either::Either<String, i32>>(&Default::default()).unwrap(), @r#"string | number"#);
    insta::assert_snapshot!(inline::<either::Either<i16, i32>>(&Default::default()).unwrap(), @r#"number"#);

    insta::assert_snapshot!(inline::<Any>(&Default::default()).unwrap(), @r#"any"#);

    insta::assert_snapshot!(inline::<MyEmptyInput>(&Default::default()).unwrap(), @"Record<string, never>");
    insta::assert_snapshot!(
        export::<MyEmptyInput>(&Default::default()).unwrap(),
        @"export type MyEmptyInput = Record<string, never>;"
    );

    // https://github.com/oscartbeaumont/specta/issues/142
    #[allow(unused_parens)]
    {
        insta::assert_snapshot!(inline::<(String)>(&Default::default()).unwrap(), @r#"string"#);
        insta::assert_snapshot!(inline::<(String,)>(&Default::default()).unwrap(), @r#"[string]"#);
    }

    // https://github.com/oscartbeaumont/specta/issues/148
    insta::assert_snapshot!(inline::<ExtraBracketsInTupleVariant>(&Default::default()).unwrap(), @"{ A: string }");
    insta::assert_snapshot!(inline::<ExtraBracketsInUnnamedStruct>(&Default::default()).unwrap(), @"string");

    // https://github.com/oscartbeaumont/specta/issues/156
    insta::assert_snapshot!(inline::<Vec<MyEnum>>(&Default::default()).unwrap(), @r#"({ A: string } | { B: number })[]"#);

    insta::assert_snapshot!(inline::<InlineTuple>(&Default::default()).unwrap(), @r#"{ demo: [string, boolean] }"#);
    insta::assert_snapshot!(
        inline::<InlineTuple2>(&Default::default()).unwrap(),
        @r#"{ demo: [{ demo: [string, boolean] }, boolean] }"#
    );

    // https://github.com/oscartbeaumont/specta/issues/220
    insta::assert_snapshot!(inline::<Box<str>>(&Default::default()).unwrap(), @r#"string"#);

    insta::assert_snapshot!(
        inline::<SkippedFieldWithinVariant>(&Default::default()).unwrap(),
        @r#"{ type: "A" } | { type: "B"; data: string }"#
    );

    // https://github.com/oscartbeaumont/specta/issues/239
    insta::assert_snapshot!(inline::<KebabCase>(&Default::default()).unwrap(), @r#"{ "test-ing": string }"#);

    // https://github.com/specta-rs/specta/issues/281
    insta::assert_snapshot!(inline::<&[&str]>(&Default::default()).unwrap(), @"string[]");
    insta::assert_snapshot!(inline::<Issue281<'_>>(&Default::default()).unwrap(), @"{ default_unity_arguments: string[] }");

    // https://github.com/oscartbeaumont/specta/issues/90
    insta::assert_snapshot!(inline::<RenameWithWeirdCharsField>(&Default::default()).unwrap(), @r#"{ "@odata.context": string }"#);
    insta::assert_snapshot!(
        inline::<RenameWithWeirdCharsVariant>(&Default::default()).unwrap(),
        @r#"{ "@odata.context": string }"#
    );
    // TODO: Reenable these tests when they are no so flaky
    // insta::assert_snapshot!(
    //     export::<RenameWithWeirdCharsStruct>(&Default::default()).unwrap_err(),
    //     @"ExportError::InvalidName(...)"
    // );
    // insta::assert_snapshot!(
    //     export::<RenameWithWeirdCharsEnum>(&Default::default()).unwrap_err(),
    //     @"ExportError::InvalidName(...)"
    // );

    // https://github.com/specta-rs/specta/issues/374
    insta::assert_snapshot!(inline::<Issue374>(&Default::default()).unwrap(), @"{ foo?: boolean; bar?: boolean }");

    // https://github.com/specta-rs/specta/issues/386
    insta::assert_snapshot!(inline::<type_type::Type>(&Default::default()).unwrap(), @"never");
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Unit1;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Unit2 {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Unit3();

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Unit4(());

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum Unit5 {
    A,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum Unit6 {
    A(),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum Unit7 {
    A {},
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SimpleStruct {
    a: i32,
    b: String,
    c: (i32, String, RefCell<i32>),
    d: Vec<String>,
    e: Option<String>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct TupleStruct1(i32);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct TupleStruct3(i32, bool, String);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "HasBeenRenamed")]
struct RenamedStruct;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum TestEnum {
    Unit,
    Single(i32),
    Multiple(i32, i32),
    Struct { a: i32 },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct RefStruct(TestEnum);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct InlineStruct {
    ref_struct: SimpleStruct,
    val: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct InlinerStruct {
    #[specta(inline)]
    inline_this: InlineStruct,
    dont_inline_this: RefStruct,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct GenericStruct<T> {
    arg: T,
}

#[derive(Serialize, Type)]
#[specta(collect = false)]
struct FlattenEnumStruct {
    outer: String,
    #[serde(flatten)]
    inner: FlattenEnum,
}

#[derive(Serialize, Type)]
#[specta(collect = false)]
#[serde(tag = "tag", content = "test")]
enum FlattenEnum {
    One,
    Two,
    Three,
}

#[derive(Serialize, Type)]
#[specta(collect = false)]
struct OverridenStruct {
    #[specta(type = String)]
    overriden_field: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct HasGenericAlias(GenericAlias<i32>);

type GenericAlias<T> = std::collections::HashMap<T, String>;

#[derive(Serialize, Type)]
#[specta(collect = false)]
enum SkipVariant {
    A(String),
    #[serde(skip)]
    B(i32),
    #[specta(skip)]
    C(i32),
}

#[derive(Serialize, Type)]
#[specta(collect = false)]
#[serde(tag = "tag", content = "data")]
enum SkipVariant2 {
    A(String),
    #[serde(skip)]
    B(i32),
    #[specta(skip)]
    C(i32),
}

#[derive(Serialize, Type)]
#[specta(collect = false)]
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

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub enum EnumMacroAttributes {
    A(#[specta(type = String)] i32),
    #[serde(rename = "bbb")]
    B(i32),
    #[serde(rename = "cccc")]
    C(#[specta(type = i32)] String),
    D {
        #[specta(type = String)]
        a: i32,
        #[serde(rename = "bbbbbb")]
        b: i32,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct PlaceholderInnerField {
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct Recursive {
    a: i32,
    children: Vec<Recursive>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]

pub enum InlineEnumField {
    #[specta(inline)]
    A(PlaceholderInnerField),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct InlineOptionalType {
    #[specta(inline)]
    pub optional_field: Option<PlaceholderInnerField>,
}

// Regression test for https://github.com/oscartbeaumont/specta/issues/56
#[derive(Type, Serialize)]
#[specta(collect = false)]
enum Rename {
    OneWord,
    #[serde(rename = "Two words")]
    TwoWords,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
pub struct TransparentTypeInner {
    inner: String,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
pub struct TransparentType(pub(crate) TransparentTypeInner);

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
pub struct TransparentType2(pub(crate) ());

#[derive(Serialize)]
pub struct NonTypeType;

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
pub struct TransparentTypeWithOverride(#[specta(type = String)] NonTypeType);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub enum BasicEnum {
    A,
    B,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum NestedEnum {
    A(String),
    B(i32),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
pub struct FlattenOnNestedEnum {
    id: String,
    #[serde(flatten)]
    result: NestedEnum,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
pub struct EnumReferenceRecordKey {
    a: HashMap<BasicEnum, i32>,
}

// https://github.com/oscartbeaumont/specta/issues/88
#[derive(Default, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub(super) struct MyEmptyInput {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[allow(unused_parens)]
pub enum ExtraBracketsInTupleVariant {
    A((String)),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[allow(unused_parens)]
pub struct ExtraBracketsInUnnamedStruct((String));

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[allow(unused_parens)]
pub struct RenameWithWeirdCharsField {
    #[serde(rename = "@odata.context")]
    odata_context: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[allow(unused_parens)]
pub enum RenameWithWeirdCharsVariant {
    #[serde(rename = "@odata.context")]
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "@odata.context")]
pub struct RenameWithWeirdCharsStruct(String);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "@odata.context")]
pub enum RenameWithWeirdCharsEnum {}

#[derive(Type, Serialize, Deserialize)]
pub enum MyEnum {
    A(String),
    B(u32),
}

#[derive(Type, Serialize, Deserialize)]
pub struct InlineTuple {
    #[specta(inline)]
    demo: (String, bool),
}

#[derive(Type, Serialize, Deserialize)]
pub struct InlineTuple2 {
    #[specta(inline)]
    demo: (InlineTuple, bool),
}

#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SkippedFieldWithinVariant {
    A(#[serde(skip)] String),
    B(String),
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct KebabCase {
    test_ing: String,
}

// https://github.com/specta-rs/specta/issues/281
#[derive(Type)]
pub struct Issue281<'a> {
    default_unity_arguments: &'a [&'a str],
}

/// https://github.com/specta-rs/specta/issues/374
#[derive(Type, Serialize)]
struct Issue374 {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    foo: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    bar: bool,
}

// https://github.com/specta-rs/specta/issues/386
// We put this test in a separate module because the parent module has `use specta::Type`,
// so it clashes with our user-defined `Type`.
mod type_type {
    #[derive(specta::Type)]
    pub enum Type {}

    #[test]
    fn typescript_types() {
        assert_ts!(Type, "never");
    }
}
