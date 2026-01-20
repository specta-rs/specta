#![allow(deprecated)]

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

        // Test `selection!`
        {
            #[derive(Clone)]
            #[allow(dead_code)]
            struct User {
                pub id: i32,
                pub name: &'static str,
                pub email: &'static str,
                pub age: i32,
                pub password: &'static str,
            }

            fn register<T: specta::Type>(types: &mut specta::TypeCollection, _: T) {
                types.register_mut::<T>();
            }
            let user = User {
                id: 1,
                name: "Monty Beaumont".into(),
                email: "monty@otbeaumont.me".into(),
                age: 7,
                password: "password123".into(),
            };

            let s1 = specta_util::selection!(user.clone(), { name, age } as UserSelection);
            assert_eq!(s1.name, "Monty Beaumont");
            assert_eq!(s1.age, 7);
            register(&mut types, s1);

            let s2 = specta_util::selection!(vec![user; 3], [{ name, age }] as UserListSelection);
            assert_eq!(s2[0].name, "Monty Beaumont");
            assert_eq!(s2[0].age, 7);
            register(&mut types, s2);
        }

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
        Vec<i32>, &'static [i32], &'static [i32; 3], [i32; 3],
        Vec<MyEnum>, &'static [MyEnum], &'static [MyEnum; 6], [MyEnum; 2],
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

        // https://github.com/specta-rs/specta/issues/171
        ActualType,

        SpectaTypeOverride,
        InvalidToValidType,

        // `#[specta(transparent)]`
        TupleStruct,
        TupleStructWithRep,
        GenericTupleStruct<String>,
        BracedStruct,

        // `#[serde(rename)]`
        // Struct,
        // Struct2,
        // Enum,
        // Enum2,
        // Enum3, // TODO: Fix these

        // Recursive types
        Recursive,
        // RecursiveMapKey, // TODO: Fix this
        RecursiveMapValue,
        RecursiveTransparent,
        RecursiveInEnum,

        // `#[serde(optional)]`
        NonOptional,
        OptionalOnNamedField,
        OptionalOnTransparentNamedField,
        OptionalInEnum,

        // Valid Map keys
        HashMap<String, ()>,
        Regular,
        HashMap<Infallible, ()>,
        // HashMap<Any, ()>, // TODO: Fix this
        // HashMap<TransparentStruct, ()>, // TODO: Fix this
        HashMap<UnitVariants, ()>,
        HashMap<UntaggedVariants, ()>,
        // ValidMaybeValidKey, // TODO: Fix this
        // ValidMaybeValidKeyNested, // TODO: Fix this
        // HashMap<() /* `null` */, ()>, // TODO: Fix this
        // HashMap<RegularStruct, ()>, // TODO: Fix this
        HashMap<Variants, ()>,
        // InvalidMaybeValidKey, // TODO: Fix this
        // InvalidMaybeValidKeyNested, // TODO: Fix this

        // `macro_rules!` in decl
        MacroStruct,
        MacroStruct2,
        MacroEnum,

        // Deprecated
        DeprecatedType,
        DeprecatedTypeWithMsg,
        DeprecatedTypeWithMsg2,
        DeprecatedFields,
        DeprecatedTupleVariant,
        DeprecatedEnumVariants,

        // Comments
        CommentedStruct,
        CommentedEnum,
        SingleLineComment,

        // Type aliases
        NonGeneric,
        HalfGenericA<u8>,
        HalfGenericB<bool>,
        FullGeneric<u8, bool>,
        Another<bool>,
        MapA<u32>,
        MapB<u32>,
        MapC<u32>,
        AGenericStruct<u32>,

        A,
        DoubleFlattened,
        FlattenedInner, // TODO: Fix this
        BoxFlattened, // TODO: Fix this
        BoxInline, // TODO: Fix this

        // Flatten and inline
        First,
        Second,
        Third,
        Fourth,
        Fifth,
        Sixth,
        Seventh,
        Eight,
        // Ninth, // TODO: Fix this
        Tenth,

        // Test for issue #393 - flatten in enum variants
        MyEnumTagged,
        MyEnumExternal,
        MyEnumAdjacent,
        MyEnumUntagged,
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

#[derive(Type, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GenericType<T> {
    Undefined,
    Value(T),
}

#[derive(Type, Serialize, Deserialize)]
pub struct ActualType {
    a: GenericType<String>,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct SpectaTypeOverride {
    #[specta(type = String)] // Ident
    string_ident: (),
    #[specta(type = u32)] // Ident
    u32_ident: (),
    #[specta(type = ::std::string::String)] // Path
    path: (),
}

// Checking that you can override the type of a field that is invalid. This is to ensure user code can override Specta in the case we have a bug/unsupported type.
#[derive(Type)]
#[specta(collect = false)]
pub struct InvalidToValidType {
    #[specta(type = Option<()>)]
    pub(crate) cause: Option<Box<dyn std::error::Error + Send + Sync>>,
}

#[derive(Type)]
#[specta(collect = false, transparent)]
struct TupleStruct(String);

#[repr(transparent)]
#[derive(Type)]
#[specta(collect = false)]
struct TupleStructWithRep(String);

#[derive(Type)]
#[specta(collect = false, transparent)]
struct GenericTupleStruct<T>(T);

#[derive(Type)]
#[specta(collect = false, transparent)]
pub struct BracedStruct {
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "StructNew", tag = "t")]
pub struct Struct {
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct Struct2 {
    #[serde(rename = "b")]
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "EnumNew", tag = "t")]
pub enum Enum {
    A,
    B,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "EnumNew", tag = "t")]
pub enum Enum2 {
    #[serde(rename = "C")]
    A,
    B,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "EnumNew", tag = "t")]
pub enum Enum3 {
    A {
        #[serde(rename = "b")]
        a: String,
    },
}

#[derive(Type)]
#[specta(collect = false)]
pub struct Recursive {
    demo: Box<Recursive>,
}

#[derive(Type)]
#[specta(transparent, collect = false)]
pub struct RecursiveMapKeyTrick(RecursiveMapKey);

#[derive(Type)]
#[specta(collect = false)]
pub struct RecursiveMapKey {
    demo: HashMap<RecursiveMapKeyTrick, String>,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct RecursiveMapValue {
    demo: HashMap<String, RecursiveMapValue>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct RecursiveInline {
    #[serde(flatten)]
    demo: Box<RecursiveInline>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(transparent, collect = false)]
pub struct RecursiveTransparent(Box<RecursiveInline>);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub enum RecursiveInEnum {
    A {
        #[serde(flatten)]
        demo: Box<RecursiveInEnum>,
    },
}

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

// Export needs a `NamedDataType` but uses `Type::reference` instead of `Type::inline` so we test it.
#[derive(Type, Serialize)]
#[specta(collect = false)]
struct Regular(HashMap<String, ()>);

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct RegularStruct {
    a: String,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct TransparentStruct(String);

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum UnitVariants {
    A,
    B,
    C,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedVariants {
    A(String),
    B(i32),
    C(u8),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum InvalidUntaggedVariants {
    A(String),
    B(i32, String),
    C(u8),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum Variants {
    A(String),
    B(i32),
    C(u8),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
pub struct MaybeValidKey<T>(T);

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
pub struct ValidMaybeValidKey(HashMap<MaybeValidKey<String>, ()>);

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
pub struct ValidMaybeValidKeyNested(HashMap<MaybeValidKey<MaybeValidKey<String>>, ()>);

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
pub struct InvalidMaybeValidKey(HashMap<MaybeValidKey<()>, ()>);

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
pub struct InvalidMaybeValidKeyNested(HashMap<MaybeValidKey<MaybeValidKey<()>>, ()>);

macro_rules! field_ty_macro {
    () => {
        String
    };
}

#[derive(Type)]
#[specta(collect = false)]
pub struct MacroStruct(field_ty_macro!());

#[derive(Type)]
#[specta(collect = false)]
pub struct MacroStruct2 {
    demo: field_ty_macro!(),
}

#[derive(Type)]
#[specta(collect = false)]
pub enum MacroEnum {
    Demo(field_ty_macro!()),
    Demo2 { demo2: field_ty_macro!() },
}

#[derive(Type)]
#[specta(collect = false)]
#[deprecated]
struct DeprecatedType {
    a: i32,
}

#[derive(Type)]
#[specta(collect = false)]
#[deprecated = "Look at you big man using a deprecation message"]
struct DeprecatedTypeWithMsg {
    a: i32,
}

#[derive(Type)]
#[specta(collect = false)]
#[deprecated(note = "Look at you big man using a deprecation message")]
struct DeprecatedTypeWithMsg2 {
    a: i32,
}

#[derive(Type)]
#[specta(collect = false)]
struct DeprecatedFields {
    a: i32,
    #[deprecated]
    b: String,
    #[deprecated = "This field is cringe!"]
    c: String,
    #[deprecated(note = "This field is cringe!")]
    d: String,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct DeprecatedTupleVariant(
    #[deprecated] String,
    #[deprecated = "Nope"] String,
    #[deprecated(note = "Nope")] i32,
);

#[derive(Type)]
#[specta(collect = false)]
pub enum DeprecatedEnumVariants {
    #[deprecated]
    A,
    #[deprecated = "Nope"]
    B,
    #[deprecated(note = "Nope")]
    C,
}

// Some double-slash comment which is ignored
/// Some triple-slash comment
/// Some more triple-slash comment
#[derive(Type)]
#[specta(collect = false)]
pub struct CommentedStruct {
    // Some double-slash comment which is ignored
    /// Some triple-slash comment
    /// Some more triple-slash comment
    a: i32,
}

// Some double-slash comment which is ignored
/// Some triple-slash comment
/// Some more triple-slash comment
#[derive(Type)]
#[specta(collect = false)]
pub enum CommentedEnum {
    // Some double-slash comment which is ignored
    /// Some triple-slash comment
    /// Some more triple-slash comment
    A(i32),
    // Some double-slash comment which is ignored
    /// Some triple-slash comment
    /// Some more triple-slash comment
    B {
        // Some double-slash comment which is ignored
        /// Some triple-slash comment
        /// Some more triple-slash comment
        a: i32,
    },
}

/// Some single-line comment
#[derive(Type)]
#[specta(collect = false)]
pub enum SingleLineComment {
    /// Some single-line comment
    A(i32),
    /// Some single-line comment
    B {
        /// Some single-line comment
        a: i32,
    },
}

#[derive(Type)]
#[specta(collect = false)]
struct Demo<A, B> {
    a: A,
    b: B,
}

type NonGeneric = Demo<u8, bool>;
type HalfGenericA<T> = Demo<T, bool>;
type HalfGenericB<T> = Demo<u8, T>;
type FullGeneric<T, U> = Demo<T, U>;

type Another<T> = FullGeneric<u8, T>;

type MapA<A> = HashMap<String, A>;
type MapB<B> = HashMap<B, String>;
type MapC<B> = HashMap<String, AGenericStruct<B>>;

#[derive(Type)]
#[specta(collect = false)]
struct AGenericStruct<T> {
    field: HalfGenericA<T>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct D {
    flattened: u32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct GenericFlattened<T> {
    generic_flattened: T,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct C {
    a: u32,
    #[serde(flatten)]
    b: D,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct B {
    b: u32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct A {
    a: B,
    #[specta(inline)]
    b: B,
    c: B,
    #[specta(inline)]
    d: D,
    #[specta(inline)]
    e: GenericFlattened<u32>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ToBeFlattened {
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct DoubleFlattened {
    a: ToBeFlattened,
    b: ToBeFlattened,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Inner {
    a: i32,
    b: Box<FlattenedInner>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenedInner {
    c: Inner,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct BoxedInner {
    a: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct BoxFlattened {
    b: Box<BoxedInner>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct BoxInline {
    #[specta(inline)]
    c: Box<BoxedInner>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct First {
    pub a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct Second {
    pub a: i32,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
pub struct Third {
    #[serde(flatten)]
    pub a: First,
    pub b: HashMap<String, String>,
    pub c: Box<First>,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct Fourth {
    pub a: First,
    #[specta(inline)]
    pub b: First,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
pub struct Fifth {
    pub a: First,
    #[specta(inline)]
    pub b: First,
}

// Flattening a struct multiple times
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct Sixth {
    pub a: First,
    pub b: First,
}

// Two fields with the same name (`a`) but different types
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct Seventh {
    pub a: First,
    pub b: Second,
}

// Serde can't serialize this
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub enum Eight {
    A(String),
    B,
}

// Test for issue #393 - flatten in enum variant with internal tag
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
pub enum MyEnumTagged {
    Variant {
        #[serde(flatten)]
        inner: First,
    },
}

// Test for issue #393 - flatten in enum variant with external tag
#[derive(Type, Serialize)]
#[specta(collect = false)]
pub enum MyEnumExternal {
    Variant {
        #[serde(flatten)]
        inner: First,
    },
}

// Test for issue #393 - flatten in enum variant with adjacent tag
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
pub enum MyEnumAdjacent {
    Variant {
        #[serde(flatten)]
        inner: First,
    },
}

// Test for issue #393 - flatten in enum variant with untagged
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
pub enum MyEnumUntagged {
    Variant {
        #[serde(flatten)]
        inner: First,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
pub enum Ninth {
    A(String),
    B,
    #[specta(inline)]
    C(First),
    D(First),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
pub enum Tenth {
    A(String),
    B,
    #[specta(inline)]
    C(First),
    D(First),
}
