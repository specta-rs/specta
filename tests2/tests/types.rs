use std::{
    cell::RefCell,
    collections::HashMap,
    convert::Infallible,
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    ops::{Range, RangeInclusive},
    path::PathBuf,
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};
use specta::{Type, TypeCollection, datatype::DataType};

/// A macro to collect up the types for better testing.
///
/// In a real-world application you should prefer the `TypeCollection::register` method instead of this.
/// In this case we can't use it because we intent to test `NamedDataType` and `DataType`'s.
/// `TypeCollection` only registers `NamedDataType` as those are the only types that aren't built-in.
macro_rules! types {
    ($($t:ty),* $(,)?) => {{
        let mut types = specta::TypeCollection::default();
        let mut dts = Vec::new();
        let mut s = specta::datatype::Struct::named();
        let mut i = 0;

        $({
            let ty = <$t as specta::Type>::definition(&mut types);

            // Like `TypeCollection::register` we are relying on the side-effect of `definition`.
            // but unlike it also storing the resulting `DataType` for testing the primitives.
            dts.push((stringify!($t), ty.clone()));

            i += 1;
            s = s.field(format!("{i:x}"), specta::datatype::Field::new(ty));
        })*

        // This allows us to end-to-end test primitives.
        // Many types won't be directly added to the `TypeCollection`, as they are not named.
        specta::datatype::NamedDataTypeBuilder::new("Primitives", vec![], s.build())
            .build(&mut types);

        (types, dts)
    }};
}

#[rustfmt::skip]
pub fn types() -> (TypeCollection, Vec<(&'static str, DataType)>) {
    types!(
        i8, i16, i32, i64, i128, isize,
        u8, u16, u32, u64, u128, usize,
        f32, f64, bool, char,

        // Serde is so mega cringe for this. Lack of support and the fact that `0..5` == `0..=5` is so dumb.
        Range<i32>, // 0..5,
        // 0..,
        // ..,
        RangeInclusive<i32>, // 0..=5,
        // ..5,
        // ..=5,

        (),
        (String, i32),
        (String, i32, bool),
        (bool, bool, bool, bool, bool, bool, bool, bool, bool, bool, bool, bool),

        String, PathBuf,
        IpAddr, Ipv4Addr, Ipv6Addr,
        SocketAddr, SocketAddrV4, SocketAddrV6,

        // https://github.com/specta-rs/specta/issues/77
        SystemTime, Duration,

        &'static str, &'static bool, &'static i32,
        Vec<i32>, &'static [i32], &'static [i32; 3],
        Vec<MyEnum>, &'static [MyEnum], &'static [MyEnum; 6],
        &'static [i32; 1], &'static [i32; 0],
        Option<i32>, Option<()>,

        // https://github.com/specta-rs/specta/issues/88
        Unit1, Unit2, Unit3, Unit4, Unit5, Unit6, Unit7,

        SimpleStruct,
        TupleStruct1,
        TupleStruct3,

        TestEnum,
        RefStruct,

        InlinerStruct,

        GenericStruct<i32>,
        GenericStruct<String>,

        FlattenEnumStruct,

        OverridenStruct,
        HasGenericAlias,

        SkipVariant,
        SkipVariant2,
        SkipVariant3,

        EnumMacroAttributes,

        Recursive,

        InlineEnumField,

        InlineOptionalType,

        Rename,

        TransparentType,
        TransparentType2,
        TransparentTypeWithOverride,

        // https://github.com/specta-rs/specta/issues/66
        [Option<u8>; 3],

        // https://github.com/specta-rs/specta/issues/65
        HashMap<BasicEnum, ()>,

        // https://github.com/specta-rs/specta/issues/60
        Option<Option<Option<Option<i32>>>>,

        // https://github.com/specta-rs/specta/issues/71
        Vec<PlaceholderInnerField>,

        HashMap<BasicEnum, i32>,
        EnumReferenceRecordKey,

        FlattenOnNestedEnum,

        PhantomData<()>,
        PhantomData<String>,
        Infallible,

        MyEmptyInput,

        // https://github.com/specta-rs/specta/issues/142
        (String),
        (String,),

        // https://github.com/specta-rs/specta/issues/148
        ExtraBracketsInTupleVariant,
        ExtraBracketsInUnnamedStruct,

        // https://github.com/specta-rs/specta/issues/156
        Vec<MyEnum>,

        InlineTuple,
        InlineTuple2,

        // https://github.com/specta-rs/specta/issues/220
        Box<str>,
        Box<String>,

        SkippedFieldWithinVariant,

        // https://github.com/specta-rs/specta/issues/239
        KebabCase,

        // https://github.com/specta-rs/specta/issues/281
        &[&str],
        Issue281<'_>,

        // https://github.com/specta-rs/specta/issues/90
        RenameWithWeirdCharsField,
        RenameWithWeirdCharsVariant,

        // https://github.com/specta-rs/specta/issues/374
        Issue374,

        // https://github.com/specta-rs/specta/issues/386
        type_type::Type,
    )
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

// Regression test for https://github.com/specta-rs/specta/issues/56
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

// https://github.com/specta-rs/specta/issues/88
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
}
