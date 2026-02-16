#![allow(deprecated)]

use std::{
    any::Any,
    borrow::Cow,
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    convert::Infallible,
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    ops::{Range, RangeInclusive},
    path::PathBuf,
    rc::Rc,
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
                id: i32,
                name: &'static str,
                email: &'static str,
                age: i32,
                password: &'static str,
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
        ((String, i32), (bool, char, bool), ()),
        (bool, bool, bool, bool, bool, bool, bool, bool, bool, bool, bool, bool),
        (Vec<i32>, Vec<bool>),

        String, PathBuf,
        IpAddr, Ipv4Addr, Ipv6Addr,
        SocketAddr, SocketAddrV4, SocketAddrV6,
        Cow<'static, str>, Cow<'static, i32>,

        // https://github.com/specta-rs/specta/issues/77
        SystemTime, Duration,

        &'static str, &'static bool, &'static i32,
        Vec<i32>, &'static [i32], &'static [i32; 3], [i32; 3],
        Vec<MyEnum>, &'static [MyEnum], &'static [MyEnum; 6], [MyEnum; 2],
        &'static [i32; 1], &'static [i32; 0],
        Option<i32>, Option<()>, Option<Vec<i32>>,
        Vec<Option<Cow<'static, i32>>>, Option<Vec<Cow<'static, i32>>>, [Vec<String>; 3],

        Option<Option<String>>,
        Option<Option<Option<String>>>,

        PhantomData<()>,
        PhantomData<String>,
        Infallible,

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
        StructRenameAllUppercase,
        RenameSerdeSpecialChar,
        EnumRenameAllUppercase,

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
        Optional,

        UntaggedVariants,
        UntaggedVariantsWithoutValue,

        // Valid Map keys
        HashMap<String, ()>,
        Regular,
        HashMap<Infallible, ()>,
        // HashMap<Any, ()>, // TODO: Fix this
        // HashMap<TransparentStruct, ()>, // TODO: Fix this
        HashMap<UnitVariants, ()>,
        HashMap<UntaggedVariantsKey, ()>,
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
        TagOnStructWithInline,
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

        // https://github.com/specta-rs/specta/issues/174
        EmptyStruct,
        EmptyStructWithTag,

        // Serde - Adjacently Tagged
        AdjacentlyTagged,
        LoadProjectEvent,

        // Serde - Externally Tagged
        ExternallyTagged,

        // Serde - Internally Tagged
        InternallyTaggedB,
        InternallyTaggedC,
        InternallyTaggedD,
        InternallyTaggedE,
        InternallyTaggedF,
        InternallyTaggedG,
        InternallyTaggedH,
        InternallyTaggedI,
        InternallyTaggedL,
        InternallyTaggedM,

        // Alias
        StructWithAlias,
        StructWithMultipleAliases,
        StructWithAliasAndRename,

        EnumWithVariantAlias,
        EnumWithMultipleVariantAliases,
        EnumWithVariantAliasAndRename,

        InternallyTaggedWithAlias,
        AdjacentlyTaggedWithAlias,
        UntaggedWithAlias,

        // https://github.com/specta-rs/specta/issues/174
        // `never & { tag = "a" }` would coalesce to `never` so we don't need to include it.
        EmptyEnum,
        EmptyEnumTagged,
        EmptyEnumTaggedWContent,
        EmptyEnumUntagged,

        TaggedEnumOfUnitStruct,
        TaggedEnumOfEmptyBracedStruct,
        TaggedEnumOfEmptyTupleStruct,
        TaggedEnumOfEmptyTupleBracedStructs,
        TaggedStructOfStructWithTuple,

        // Skip
        SkipOnlyField,
        SkipField,
        SkipOnlyVariantExternallyTagged,
        SkipOnlyVariantInternallyTagged,
        SkipOnlyVariantAdjacentlyTagged,
        SkipOnlyVariantUntagged,
        SkipVariant,
        SkipUnnamedFieldInVariant,
        SkipNamedFieldInVariant,
        TransparentWithSkip,
        TransparentWithSkip2,
        TransparentWithSkip3,
        SkipVariant2,
        SkipVariant3,
        SkipStructFields,
        SpectaSkipNonTypeField,

        // Flatten
        FlattenA,
        FlattenB,
        FlattenC,
        FlattenD,
        FlattenE,
        FlattenF,
        FlattenG,

        // Generic Fields
        TupleNested,

        // Generic
        Generic1<()>,
        GenericAutoBound<()>,
        GenericAutoBound2<()>,
        Container1,
        Generic2<(), String, i32>,
        GenericNewType1<()>,
        GenericTuple<()>,
        GenericStruct2<()>,
        InlineGenericNewtype<String>,
        InlineGenericNested<String>,
        InlineFlattenGenericsG<()>,
        InlineFlattenGenerics,
        GenericParameterOrderPreserved,
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

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct SkipStructFields {
    a: i32,
    #[specta(skip)]
    b: String,
    #[serde(skip)]
    d: Box<dyn std::any::Any>, // `!Type`
}

#[derive(Type)]
#[specta(collect = false)]
struct SpectaSkipNonTypeField {
    a: i32,
    #[specta(skip)]
    d: Box<dyn std::any::Any>, // `!Type`
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum EnumMacroAttributes {
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
struct PlaceholderInnerField {
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum InlineEnumField {
    #[specta(inline)]
    A(PlaceholderInnerField),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct InlineOptionalType {
    #[specta(inline)]
    optional_field: Option<PlaceholderInnerField>,
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
struct TransparentTypeInner {
    inner: String,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct TransparentType(TransparentTypeInner);

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct TransparentType2(());

#[derive(Serialize)]
struct NonTypeType;

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct TransparentTypeWithOverride(#[specta(type = String)] NonTypeType);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum BasicEnum {
    A,
    B,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
enum NestedEnum {
    A(String),
    B(i32),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
struct FlattenOnNestedEnum {
    id: String,
    #[serde(flatten)]
    result: NestedEnum,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct EnumReferenceRecordKey {
    a: HashMap<BasicEnum, i32>,
}

// https://github.com/specta-rs/specta/issues/88
#[derive(Default, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
struct MyEmptyInput {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct EmptyStruct {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
struct EmptyStructWithTag {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[allow(unused_parens)]
enum ExtraBracketsInTupleVariant {
    A((String)),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ExtraBracketsInUnnamedStruct((String));

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[allow(unused_parens)]
struct RenameWithWeirdCharsField {
    #[serde(rename = "@odata.context")]
    odata_context: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[allow(unused_parens)]
enum RenameWithWeirdCharsVariant {
    #[serde(rename = "@odata.context")]
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "@odata.context")]
struct RenameWithWeirdCharsStruct(String);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "@odata.context")]
enum RenameWithWeirdCharsEnum {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum MyEnum {
    A(String),
    B(u32),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct InlineTuple {
    #[specta(inline)]
    demo: (String, bool),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct InlineTuple2 {
    #[specta(inline)]
    demo: (InlineTuple, bool),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type", content = "data")]
enum SkippedFieldWithinVariant {
    A(#[serde(skip)] String),
    B(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "kebab-case")]
struct KebabCase {
    test_ing: String,
}

// https://github.com/specta-rs/specta/issues/281
#[derive(Type)]
#[specta(collect = false)]
struct Issue281<'a> {
    default_unity_arguments: &'a [&'a str],
}

/// https://github.com/specta-rs/specta/issues/374
#[derive(Type, Serialize)]
#[specta(collect = false)]
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
    #[specta(collect = false)]
    pub(super) enum Type {}
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum GenericType<T> {
    Undefined,
    Value(T),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ActualType {
    a: GenericType<String>,
}

#[derive(Type)]
#[specta(collect = false)]
struct SpectaTypeOverride {
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
struct InvalidToValidType {
    #[specta(type = Option<()>)]
    cause: Option<Box<dyn std::error::Error + Send + Sync>>,
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
struct BracedStruct {
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "StructNew", tag = "t")]
struct Struct {
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Struct2 {
    #[serde(rename = "b")]
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "EnumNew", tag = "t")]
enum Enum {
    A,
    B,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "EnumNew", tag = "t")]
enum Enum2 {
    #[serde(rename = "C")]
    A,
    B,
    #[serde(rename_all = "camelCase")]
    D {
        enum_field: (),
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "UPPERCASE")]
struct StructRenameAllUppercase {
    a: i32,
    b: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "UPPERCASE")]
enum EnumRenameAllUppercase {
    HelloWorld,
    VariantB,
    TestingWords,
}

#[derive(serde::Serialize, Type)]
#[specta(collect = false)]
struct RenameSerdeSpecialChar {
    #[serde(rename = "a/b")]
    b: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "EnumNew", tag = "t")]
enum Enum3 {
    A {
        #[serde(rename = "b")]
        a: String,
    },
}

#[derive(Type)]
#[specta(collect = false)]
struct Recursive {
    demo: Box<Recursive>,
}

#[derive(Type)]
#[specta(transparent, collect = false)]
struct RecursiveMapKeyTrick(RecursiveMapKey);

#[derive(Type)]
#[specta(collect = false)]
struct RecursiveMapKey {
    demo: HashMap<RecursiveMapKeyTrick, String>,
}

#[derive(Type)]
#[specta(collect = false)]
struct RecursiveMapValue {
    demo: HashMap<String, RecursiveMapValue>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct RecursiveInline {
    #[serde(flatten)]
    demo: Box<RecursiveInline>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(transparent, collect = false)]
struct RecursiveTransparent(Box<RecursiveInline>);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum RecursiveInEnum {
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
enum UntaggedVariantsKey {
    A(String),
    B(i32),
    C(u8),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedVariants {
    A(String),
    B(i32),
    C(u8),
    D { id: String },
    E(String, bool),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedVariantsWithoutValue {
    A(String),
    B(i32, String),
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
struct MaybeValidKey<T>(T);

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct ValidMaybeValidKey(HashMap<MaybeValidKey<String>, ()>);

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct ValidMaybeValidKeyNested(HashMap<MaybeValidKey<MaybeValidKey<String>>, ()>);

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct InvalidMaybeValidKey(HashMap<MaybeValidKey<()>, ()>);

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct InvalidMaybeValidKeyNested(HashMap<MaybeValidKey<MaybeValidKey<()>>, ()>);

macro_rules! field_ty_macro {
    () => {
        String
    };
}

#[derive(Type)]
#[specta(collect = false)]
struct MacroStruct(field_ty_macro!());

#[derive(Type)]
#[specta(collect = false)]
struct MacroStruct2 {
    demo: field_ty_macro!(),
}

#[derive(Type)]
#[specta(collect = false)]
enum MacroEnum {
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
struct DeprecatedTupleVariant(
    #[deprecated] String,
    #[deprecated = "Nope"] String,
    #[deprecated(note = "Nope")] i32,
);

#[derive(Type)]
#[specta(collect = false)]
enum DeprecatedEnumVariants {
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
struct CommentedStruct {
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
enum CommentedEnum {
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
enum SingleLineComment {
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
struct First {
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Second {
    a: i32,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct Third {
    #[serde(flatten)]
    a: First,
    b: HashMap<String, String>,
    c: Box<First>,
}

#[derive(Type)]
#[specta(collect = false)]
struct Fourth {
    a: First,
    #[specta(inline)]
    b: First,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
struct TagOnStructWithInline {
    a: First,
    #[specta(inline)]
    b: First,
}

// Flattening a struct multiple times
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Sixth {
    a: First,
    b: First,
}

// Two fields with the same name (`a`) but different types
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Seventh {
    a: First,
    b: Second,
}

// Serde can't serialize this
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum Eight {
    A(String),
    B,
}

// Test for issue #393 - flatten in enum variant with internal tag
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum MyEnumTagged {
    Variant {
        #[serde(flatten)]
        inner: First,
    },
}

// Test for issue #393 - flatten in enum variant with external tag
#[derive(Type, Serialize)]
#[specta(collect = false)]
enum MyEnumExternal {
    Variant {
        #[serde(flatten)]
        inner: First,
    },
}

// Test for issue #393 - flatten in enum variant with adjacent tag
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum MyEnumAdjacent {
    Variant {
        #[serde(flatten)]
        inner: First,
    },
}

// Test for issue #393 - flatten in enum variant with untagged
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum MyEnumUntagged {
    Variant {
        #[serde(flatten)]
        inner: First,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum Ninth {
    A(String),
    B,
    #[specta(inline)]
    C(First),
    D(First),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum Tenth {
    A(String),
    B,
    #[specta(inline)]
    C(First),
    D(First),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Optional {
    a: Option<i32>,
    #[specta(optional)]
    b: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    c: Option<String>,
    #[serde(default)]
    d: bool,
}

// Test that attributes with format strings are properly parsed
// This tests the fix for parsing attributes like #[error("io error: {0}")]
// which were causing "expected ident" errors in the lower_attr.rs parser
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[allow(dead_code)]
enum TypeWithComplexAttributes {
    // These attributes will be parsed by lower_attr.rs and should not cause errors
    #[doc = "This is a variant with format-like strings in docs: {0}"]
    A(String),

    #[doc = "Another variant: {line} {msg}"]
    B { line: usize, msg: String },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjacentlyTagged {
    A,
    B { id: String, method: String },
    C(String),
}

// Test for https://github.com/specta-rs/specta/issues/395
// The `rename_all_fields = "camelCase"` should convert field names to camelCase
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
enum LoadProjectEvent {
    Started {
        project_name: String,
    },
    ProgressTest {
        project_name: String,
        status: String,
        progress: i32,
    },
    Finished {
        project_name: String,
    },
}

#[derive(Type)]
#[specta(collect = false)]
enum ExternallyTagged {
    A,
    B { id: String, method: String },
    C(String),
}

// Test struct with field alias
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct StructWithAlias {
    #[serde(alias = "bruh")]
    field: String,
}

// Test struct with multiple aliases on same field
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct StructWithMultipleAliases {
    #[serde(alias = "bruh", alias = "alternative", alias = "another")]
    field: String,
}

// Test struct with alias and rename
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct StructWithAliasAndRename {
    #[serde(rename = "renamed_field", alias = "bruh")]
    field: String,
}

// Test enum variant with alias
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum EnumWithVariantAlias {
    #[serde(alias = "bruh")]
    Variant,
    Other,
}

// Test enum with multiple variant aliases
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum EnumWithMultipleVariantAliases {
    #[serde(alias = "bruh", alias = "alternative")]
    Variant,
    Other,
}

// Test enum variant with alias and rename
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum EnumWithVariantAliasAndRename {
    #[serde(rename = "renamed_variant", alias = "bruh")]
    Variant,
    Other,
}

// Test internally tagged enum with field alias
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InternallyTaggedWithAlias {
    A {
        #[serde(alias = "bruh")]
        field: String,
    },
    B {
        other: i32,
    },
}

// Test adjacently tagged enum with field alias
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type", content = "data")]
enum AdjacentlyTaggedWithAlias {
    A {
        #[serde(alias = "bruh")]
        field: String,
    },
    B {
        other: i32,
    },
}

// Test untagged enum with field alias
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedWithAlias {
    A {
        #[serde(alias = "bruh")]
        field: String,
    },
    B {
        other: i32,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum EmptyEnum {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum EmptyEnumTagged {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a", content = "b")]
enum EmptyEnumTaggedWContent {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum EmptyEnumUntagged {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct UnitStruct;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct EmptyBracedStruct {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct EmptyTupleStruct();

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum TaggedEnumOfUnitStruct {
    A(UnitStruct),
    B(UnitStruct),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum TaggedEnumOfEmptyBracedStruct {
    A(EmptyBracedStruct),
    B(EmptyBracedStruct),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum TaggedEnumOfEmptyTupleStruct {
    A(EmptyTupleStruct),
    B(EmptyTupleStruct),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum TaggedEnumOfEmptyTupleBracedStructs {
    #[specta(skip)]
    A(EmptyTupleStruct),
    B(EmptyBracedStruct),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false, transparent)]
struct TupleStructWithTuple(());

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum TaggedStructOfStructWithTuple {
    A(TupleStructWithTuple),
    B(TupleStructWithTuple),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InternallyTaggedB {
    // Is not a map-type so invalid.
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InternallyTaggedC {
    // Is not a map-type so invalid.
    A(Vec<String>),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InternallyTaggedD {
    // Is a map type so valid.
    A(HashMap<String, String>),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InternallyTaggedE {
    // Null is valid (although it's not a map-type)
    A(()),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InternallyTaggedF {
    // `FInner` is untagged so this is *only* valid if it is (which it is)
    A(InternallyTaggedFInner),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum InternallyTaggedFInner {
    A(()),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InternallyTaggedG {
    // `GInner` is untagged so this is *only* valid if it is (which it is not)
    A(InternallyTaggedGInner),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum InternallyTaggedGInner {
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InternallyTaggedH {
    // `HInner` is transparent so this is *only* valid if it is (which it is)
    A(InternallyTaggedHInner),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct InternallyTaggedHInner(());

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InternallyTaggedI {
    // `IInner` is transparent so this is *only* valid if it is (which it is not)
    A(InternallyTaggedIInner),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct InternallyTaggedIInner(String);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InternallyTaggedL {
    // Internally tag enum with inlined field that is itself internally tagged
    #[specta(inline)]
    A(InternallyTaggedLInner),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InternallyTaggedLInner {
    A,
    B,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InternallyTaggedM {
    // Internally tag enum with inlined field that is untagged
    // `MInner` is `null` - Test `B` in `untagged.rs`
    #[specta(inline)]
    A(InternallyTaggedMInner),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum InternallyTaggedMInner {
    A,
    B,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkipOnlyField {
    #[specta(skip)]
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkipField {
    #[specta(skip)]
    a: String,
    b: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum SkipOnlyVariantExternallyTagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum SkipOnlyVariantInternallyTagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum SkipOnlyVariantAdjacentlyTagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum SkipOnlyVariantUntagged {
    #[specta(skip)]
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum SkipUnnamedFieldInVariant {
    // only field
    A(#[specta(skip)] String),
    // not only field
    //
    // This will `B(String)` == `String` in TS whether this will be `[String]`. This is why `#[serde(skip)]` is processed at runtime not in the macro.
    B(#[specta(skip)] String, i32),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum SkipNamedFieldInVariant {
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
struct TransparentWithSkip((), #[specta(skip)] String);

// https://github.com/specta-rs/specta/issues/170
#[derive(Type, Serialize, Deserialize)]
#[specta(transparent, collect = false)]
struct TransparentWithSkip2(#[specta(skip)] (), String);

// https://github.com/specta-rs/specta/issues/170
#[derive(Type)]
#[specta(transparent, collect = false)]
struct TransparentWithSkip3(#[specta(type = String)] Box<dyn Any>);

/// This is intentionally just a compile or not compile test
/// https://github.com/specta-rs/specta/issues/167
#[derive(Type, Serialize)]
#[specta(collect = false)]
enum LazilySkip {
    #[serde(skip)]
    A(Box<dyn Any>),
    B(#[serde(skip)] Box<dyn Any>),
    C {
        #[serde(skip)]
        a: Box<dyn Any>,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenA {
    a: i32,
    b: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenB {
    #[serde(flatten)]
    a: FlattenA,
    c: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenC {
    #[serde(flatten)]
    a: FlattenA,
    c: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenD {
    a: FlattenA,
    c: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenE {
    #[specta(inline)]
    b: FlattenB,
    d: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenF {
    #[specta(inline = true)]
    b: FlattenB,
    d: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenG {
    #[specta(inline = false)]
    b: FlattenB,
    d: i32,
}

#[derive(Type)]
#[specta(collect = false)]
struct TupleNested(Vec<i32>, (Vec<i32>, Vec<i32>), [Vec<i32>; 3]);

#[derive(Type)]
#[specta(collect = false)]
struct Generic1<T: Type> {
    value: T,
    values: Vec<T>,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericAutoBound<T> {
    value: T,
    values: Vec<T>,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericAutoBound2<T: PartialEq> {
    value: T,
    values: Vec<T>,
}

#[derive(Type)]
#[specta(collect = false)]
struct Container1 {
    foo: Generic1<u32>,
    bar: HashSet<Generic1<u32>>,
    baz: BTreeMap<String, Rc<Generic1<String>>>,
}

#[derive(Type)]
#[specta(collect = false)]
enum Generic2<A, B, C> {
    A(A),
    B(B, B, B),
    C(Vec<C>),
    D(Vec<Vec<Vec<A>>>),
    E { a: A, b: B, c: C },
    X(Vec<i32>),
    Y(i32),
    Z(Vec<Vec<i32>>),
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericStruct2<T> {
    a: T,
    b: (T, T),
    c: (T, (T, T)),
    d: [T; 3],
    e: [(T, T); 3],
    f: Vec<T>,
    g: Vec<Vec<T>>,
    h: Vec<[(T, T); 3]>,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericNewType1<T>(Vec<Vec<T>>);

#[derive(Type)]
#[specta(collect = false)]
struct GenericTuple<T>(T, Vec<T>, Vec<Vec<T>>);

#[derive(Type)]
#[specta(collect = false, inline)]
struct InlineGenericNewtype<T>(T);

#[derive(Type)]
#[specta(collect = false, inline)]
enum InlineGenericEnum<T> {
    Unit,
    Unnamed(T),
    Named { value: T },
}

#[derive(Type)]
#[specta(collect = false, inline)]
struct InlineGenericNested<T>(
    InlineGenericNewtype<T>,
    Vec<T>,
    (T, T),
    HashMap<String, T>,
    Option<T>,
    InlineGenericEnum<T>,
);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct InlineFlattenGenericsG<T> {
    t: T,
}

// not currently possible in ts-rs hehe
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct InlineFlattenGenerics {
    g: InlineFlattenGenericsG<String>,
    #[specta(inline)]
    gi: InlineFlattenGenericsG<String>,
    #[serde(flatten)]
    t: InlineFlattenGenericsG<String>,
}

// #[test]
// fn default() {
//     #[derive(Type)]
//     #[specta(collect = false)]
//     struct A<T = String> {
//         t: T,
//     }
//     assert_ts_export!(
//         ts_A::<()>,
//         "export type A<T = string> = { t: T, }"
//     );

//     #[derive(Type)]
//     #[specta(collect = false)]
//     struct B<U = Option<A<i32>>> {
//         u: U,
//     }
//     assert_ts_export!(
//         ts_B::<()>,
//         "export type B<U = A<number> | null>  = { u: U, }"
//     );

//     #[derive(Type)]
//     #[specta(collect = false)]
//     struct Y {
//         a1: A,
//         a2: A<i32>,
// https://github.com/Aleph-Alpha/ts-rs/issues/56
// TODO: fixme
// #[ts(inline)]
// xi: X,
// #[ts(inline)]
// xi2: X<i32>
// }
// assert_ts_export!(
//     ts_Y,
//     "type Y = { a1: A, a2: A<number> }"
// )
// }

// TODO

// #[test]
// fn test_generic_trait_bounds() {
//     #[derive(Type)]
//     struct A<T: ToString = i32> {
//         t: T,
//     }
//     assert_ts_export!(A::<i32>, "export type A<T = number> = { t: T, }");

//     #[derive(Type)]
//     struct B<T: ToString + std::fmt::Debug + Clone + 'static>(T);
//     assert_ts_export!(B::<&'static str>, "export type B<T> = T;");

//     #[derive(Type)]
//     enum C<T: Copy + Clone + PartialEq, K: Copy + PartialOrd = i32> {
//         A { t: T },
//         B(T),
//         C,
//         D(T, K),
//     }
//     assert_ts_export!(
//         C::<&'static str, i32>,
//         "export type C<T, K = number> = { A: { t: T, } } | { B: T } | \"C\" | { D: [T, K] };"
//     );

//     #[derive(Type)]
//     struct D<T: ToString, const N: usize> {
//         t: [T; N],
//     }

//     assert_ts_export!(D::<&str, 41>, "export type D<T> = { t: Array<T>, }")
// }

// https://github.com/specta-rs/specta/issues/400
#[derive(Type)]
#[specta(collect = false)]
struct Pair<Z, A> {
    first: Z,
    second: A,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericParameterOrderPreserved {
    pair: Pair<i32, String>,
}

// mod type_overrides {
//     #![allow(dead_code)]

//     use std::time::Instant;

//     use specta::Type;

//     struct Unsupported<T>(T);
//     struct Unsupported2;

//     #[test]
//     fn simple() {
//         #[derive(Type)]
//         #[specta(collect = false)]
//         struct Override {
//             a: i32,
//             #[specta(type = String)]
//             x: Instant,
//             #[specta(type = String)]
//             y: Unsupported<Unsupported<Unsupported2>>,
//             #[specta(type = Option<String>)]
//             z: Option<Unsupported2>,
//         }

//         insta::assert_snapshot!(crate::ts::inline::<Override>(&Default::default()).unwrap(), @"{ a: number; x: string; y: string; z: string | null }");
//     }

//     #[test]
//     fn newtype() {
//         #[derive(Type)]
//         #[specta(collect = false)]
//         struct New1(#[specta(type = String)] Unsupported2);
//         #[derive(Type)]
//         #[specta(collect = false)]
//         struct New2(#[specta(type = Option<String>)] Unsupported<Unsupported2>);

//         insta::assert_snapshot!(crate::ts::inline::<New1>(&Default::default()).unwrap(), @r#"string"#);
//         insta::assert_snapshot!(crate::ts::inline::<New2>(&Default::default()).unwrap(), @r#"string | null"#);
//     }
// }

// mod union_serde {
//     use serde::{Deserialize, Serialize};
//     use specta::Type;

//     #[derive(Type, Serialize, Deserialize)]
//     #[specta(collect = false)]
//     #[serde(tag = "kind", content = "d")]
//     enum SimpleEnumA {
//         A,
//         B,
//     }

//     #[derive(Type, Serialize, Deserialize)]
//     #[specta(collect = false)]
//     #[serde(tag = "kind", content = "data")]
//     enum ComplexEnum {
//         A,
//         B { foo: String, bar: f64 },
//         W(SimpleEnumA),
//         F { nested: SimpleEnumA },
//         T(i32, SimpleEnumA),
//     }

//     #[derive(Type, Serialize, Deserialize)]
//     #[specta(collect = false)]
//     #[serde(untagged)]
//     enum Untagged {
//         Foo(String),
//         Bar(i32),
//         None,
//     }

//     #[test]
//     fn test_serde_enum() {
//         insta::assert_snapshot!(crate::ts::inline::<SimpleEnumA>(&Default::default()).unwrap(), @r#"{ kind: "A" } | { kind: "B" }"#);
//         insta::assert_snapshot!(crate::ts::inline::<ComplexEnum>(&Default::default()).unwrap(), @r#"{ kind: "A" } | { kind: "B"; data: { foo: string; bar: number } } | { kind: "W"; data: SimpleEnumA } | { kind: "F"; data: { nested: SimpleEnumA } } | { kind: "T"; data: [number, SimpleEnumA] }"#);
//         insta::assert_snapshot!(crate::ts::inline::<Untagged>(&Default::default()).unwrap(), @r#"string | number | null"#);
//     }
// }

// mod union_with_serde {
//     use serde::Serialize;
//     use specta::Type;

//     #[derive(Type, Serialize)]
//     #[specta(collect = false)]
//     struct Bar {
//         field: i32,
//     }

//     #[derive(Type, Serialize)]
//     #[specta(collect = false)]
//     struct Foo {
//         bar: Bar,
//     }

//     #[derive(Type, Serialize)]
//     #[specta(collect = false)]
//     enum SimpleEnum2 {
//         A(String),
//         B(i32),
//         C,
//         D(String, i32),
//         E(Foo),
//         F { a: i32, b: String },
//     }

//     #[test]
//     fn test_stateful_enum() {
//         insta::assert_snapshot!(crate::ts::inline::<Bar>(&Default::default()).unwrap(), @r#"{ field: number }"#);

//         insta::assert_snapshot!(crate::ts::inline::<Foo>(&Default::default()).unwrap(), @r#"{ bar: Bar }"#);

//         insta::assert_snapshot!(crate::ts::inline::<SimpleEnum2>(&Default::default()).unwrap(), @r#"{ A: string } | { B: number } | "C" | { D: [string, number] } | { E: Foo } | { F: { a: number; b: string } }"#);
//     }
// }

// mod union_with_internal_tag {
//     use serde::{Deserialize, Serialize};
//     use specta::Type;

//     #[derive(Type, Serialize, Deserialize)]
//     #[specta(collect = false)]
//     #[serde(tag = "type")]
//     enum EnumWithInternalTag {
//         A { foo: String },
//         B { bar: i32 },
//     }

//     #[derive(Type, Serialize, Deserialize)]
//     #[specta(collect = false)]
//     struct InnerA {
//         foo: String,
//     }

//     #[derive(Type, Serialize, Deserialize)]
//     #[specta(collect = false)]
//     struct InnerB {
//         bar: i32,
//     }

//     #[derive(Type, Serialize, Deserialize)]
//     #[specta(collect = false)]
//     #[serde(tag = "type")]
//     enum EnumWithInternalTag2 {
//         A(InnerA),
//         B(InnerB),
//     }

//     #[test]
//     fn test_enums_with_internal_tags() {
//         insta::assert_snapshot!(crate::ts::inline::<EnumWithInternalTag>(&Default::default()).unwrap(), @r#"{ type: "A"; foo: string } | { type: "B"; bar: number }"#);

//         insta::assert_snapshot!(crate::ts::inline::<EnumWithInternalTag2>(&Default::default()).unwrap(), @r#"({ type: "A" } & InnerA) | ({ type: "B" } & InnerB)"#);
//     }
// }

// Transparent wrappers should have distinct type IDs (regression test for linker ICF bug)
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct TransparentA(String);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct TransparentB(String);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct UsesTransparent {
    a: TransparentA,
    b: TransparentB,
}

#[test]
fn transparent_wrappers_have_distinct_ids() {
    let mut types = TypeCollection::default();
    let id_a = TransparentA::definition(&mut types);
    let id_b = TransparentB::definition(&mut types);
    assert_ne!(format!("{:?}", id_a), format!("{:?}", id_b));
    assert_eq!(types.len(), 2);
}

#[test]
fn struct_collects_all_transparent_field_types() {
    let mut types = TypeCollection::default();
    UsesTransparent::definition(&mut types);
    assert_eq!(types.len(), 3); // UsesTransparent + TransparentA + TransparentB
}
