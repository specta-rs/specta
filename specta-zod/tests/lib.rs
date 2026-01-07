// use std::{
//     cell::RefCell,
//     collections::HashMap,
//     convert::Infallible,
//     marker::PhantomData,
//     net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
//     path::PathBuf,
// };

// use specta::Type;
// use specta_util::Any;
// use specta_zod::{BigIntExportBehavior, ExportConfig, ExportError, ExportPath, NamedLocation};

// macro_rules! assert_zod {
//     (error; $t:ty, $e:expr) => {
//         assert_eq!(
//             specta_zod::inline::<$t>(&Default::default()),
//             Err($e.into())
//         )
//     };
//     ($t:ty, $e:expr) => {
//         assert_eq!(specta_zod::inline::<$t>(&Default::default()), Ok($e.into()))
//     };

//     (() => $expr:expr, $e:expr) => {
//         let _: () = {
//             fn assert_ty_eq<T: Type>(_t: T) {
//                 assert_eq!(specta_zod::inline::<T>(&Default::default()), Ok($e.into()));
//             }
//             assert_ty_eq($expr);
//         };
//     };
// }
// pub(crate) use assert_zod;

// macro_rules! assert_ts_export {
//     ($t:ty, $e:expr) => {
//         assert_eq!(specta_zod::export::<$t>(&Default::default()), Ok($e.into()))
//     };
//     (error; $t:ty, $e:expr) => {
//         assert_eq!(
//             specta_zod::export::<$t>(&Default::default()),
//             Err($e.into())
//         )
//     };
//     ($t:ty, $e:expr; $cfg:expr) => {
//         assert_eq!(specta_zod::export::<$t>($cfg), Ok($e.into()))
//     };
//     (error; $t:ty, $e:expr; $cfg:expr) => {
//         assert_eq!(specta_zod::export::<$t>($cfg), Err($e.into()))
//     };
// }
// pub(crate) use assert_ts_export;

// // TODO: Unit test other `specta::Type` methods such as `::reference(...)`

// #[test]
// fn typescript_types() {
//     assert_zod!(
//         Vec<MyEnum>,
//         r#"z.array(z.union([z.object({ A: z.string() }), z.object({ B: z.number() })]))"#
//     );

//     assert_zod!(i8, "z.number()");
//     assert_zod!(u8, "z.number()");
//     assert_zod!(i16, "z.number()");
//     assert_zod!(u16, "z.number()");
//     assert_zod!(i32, "z.number()");
//     assert_zod!(u32, "z.number()");
//     assert_zod!(f32, "z.number()");
//     assert_zod!(f64, "z.number()");

//     assert_zod!(bool, "z.boolean()");

//     assert_zod!((), "z.null()");
//     assert_zod!((String, i32), "z.tuple([z.string(), z.number()])");
//     assert_zod!(
//         (String, i32, bool),
//         "z.tuple([z.string(), z.number(), z.boolean()])"
//     );
//     assert_zod!(
//         (bool, bool, bool, bool, bool, bool, bool, bool, bool, bool, bool, bool),
//         "z.tuple([z.boolean(), z.boolean(), z.boolean(), z.boolean(), z.boolean(), z.boolean(), z.boolean(), z.boolean(), z.boolean(), z.boolean(), z.boolean(), z.boolean()])"
//     );

//     assert_zod!(String, "z.string()");
//     // impossible since Path as a generic is unsized lol
//     // assert_ts!(Path, "string");
//     assert_zod!(PathBuf, "z.string()");
//     assert_zod!(IpAddr, "z.string()");
//     assert_zod!(Ipv4Addr, "z.string()");
//     assert_zod!(Ipv6Addr, "z.string()");
//     assert_zod!(SocketAddr, "z.string()");
//     assert_zod!(SocketAddrV4, "z.string()");
//     assert_zod!(SocketAddrV6, "z.string()");
//     assert_zod!(char, "z.string()");
//     assert_zod!(&'static str, "z.string()");

//     assert_zod!(&'static bool, "z.boolean()");
//     assert_zod!(&'static i32, "z.number()");

//     assert_zod!(Vec<i32>, "z.array(z.number())");
//     assert_zod!(&[i32], "z.array(z.number())");
//     assert_zod!(&[i32; 3], "z.tuple([z.number(), z.number(), z.number()])");

//     assert_zod!(Option<i32>, "z.number().nullable()");

//     // // https://github.com/specta-rs/specta/issues/88
//     assert_zod!(Unit1, "z.null()");
//     assert_zod!(Unit2, "z.record(z.string(), z.never())");
//     assert_zod!(Unit3, "z.tuple([])");
//     assert_zod!(Unit4, "z.null()");
//     assert_zod!(Unit5, r#"z.literal("A")"#);
//     assert_zod!(Unit6, r#"z.object({ A: z.tuple([]) })"#);
//     assert_zod!(Unit7, r#"z.object({ A: z.record(z.string(), z.never()) })"#);

//     assert_zod!(
//         SimpleStruct,
//         "z.object({ a: z.number(), b: z.string(), c: z.tuple([z.number(), z.string(), z.number()]), d: z.array(z.string()), e: z.string().nullable() })"
//     );
//     assert_zod!(TupleStruct1, "z.number()");
//     assert_zod!(
//         TupleStruct3,
//         "z.tuple([z.number(), z.boolean(), z.string()])"
//     );

//     assert_zod!(
//         TestEnum,
//         r#"z.union([z.literal("Unit"), z.object({ Single: z.number() }), z.object({ Multiple: z.tuple([z.number(), z.number()]) }), z.object({ Struct: z.object({ a: z.number() }) })])"#
//     );
//     assert_zod!(RefStruct, "TestEnum");

//     assert_zod!(
//         InlinerStruct,
//         r#"z.object({ inline_this: z.object({ ref_struct: SimpleStruct, val: z.number() }), dont_inline_this: RefStruct })"#
//     );

//     // TODO: Fix these
//     // assert_zod!(GenericStruct<i32>, "z.object({ arg: z.number() })");
//     // assert_zod!(GenericStruct<String>, "z.object({ arg: z.string() })");

//     assert_zod!(
//         FlattenEnumStruct,
//         r#"z.union([z.object({ tag: z.literal("One") }), z.object({ tag: z.literal("Two") }), z.object({ tag: z.literal("Three") })]).and(z.object({ outer: z.string() }))"#
//     );

//     assert_zod!(OverridenStruct, "z.object({ overriden_field: z.string() })");
//     assert_zod!(HasGenericAlias, "z.record(z.number(), z.string())");

//     assert_zod!(SkipVariant, r#"z.object({ A: z.string() })"#);
//     assert_zod!(
//         SkipVariant2,
//         r#"z.object({ tag: z.literal("A"), data: z.string() })"#
//     );
//     assert_zod!(
//         SkipVariant3,
//         r#"z.object({ A: z.object({ a: z.string() }) })"#
//     );

//     assert_zod!(
//         EnumMacroAttributes,
//         r#"z.union([z.object({ A: z.string() }), z.object({ bbb: z.number() }), z.object({ cccc: z.number() }), z.object({ D: z.object({ a: z.string(), bbbbbb: z.number() }) })])"#
//     );

//     assert_zod!(
//         Recursive,
//         "z.object({ a: z.number(), children: z.array(Recursive) })"
//     );

//     assert_zod!(
//         InlineEnumField,
//         r#"z.object({ A: z.object({ a: z.string() }) })"#
//     );

//     assert_zod!(
//         InlineOptionalType,
//         "z.object({ optional_field: PlaceholderInnerField.nullable() })"
//     );

//     assert_ts_export!(
//         RenameToValue,
//         r#"export const RenameToValueNewName = z.object({ demo_new_name: z.number() })"#
//     );

//     assert_zod!(
//         Rename,
//         r#"z.union([z.literal("OneWord"), z.literal("Two words")])"#
//     );

//     assert_zod!(TransparentType, "TransparentTypeInner"); // TODO: I don't think this is correct for `Type::inline`
//     assert_zod!(TransparentType2, "z.null()");
//     assert_zod!(TransparentTypeWithOverride, "z.string()");

//     // I love serde but this is so mega cringe. Lack of support and the fact that `0..5` == `0..=5` is so dumb.
//     assert_zod!(() => 0..5, r#"z.object({ start: z.number(), end: z.number() })"#);
//     // assert_ts!(() => 0.., r#"{ start: 0 }"#);
//     // assert_ts!(() => .., r#""#);
//     assert_zod!(() => 0..=5, r#"z.object({ start: z.number(), end: z.number() })"#);
//     // assert_ts!(() => ..5, r#"{ end: 5 }"#);
//     // assert_ts!(() => ..=5, r#"{ end: 5 }"#);

//     // https://github.com/specta-rs/specta/issues/66
//     assert_zod!(
//         [Option<u8>; 3],
//         r#"z.tuple([z.number().nullable(), z.number().nullable(), z.number().nullable()])"#
//     );

//     // https://github.com/specta-rs/specta/issues/65
//     assert_zod!(HashMap<BasicEnum, ()>, r#"z.record(z.union([z.literal("A"), z.literal("B")]), z.null())"#);

//     // https://github.com/specta-rs/specta/issues/60
//     assert_zod!(
//         Option<Option<Option<Option<i32>>>>,
//         r#"z.number().nullable()"#
//     );

//     // https://github.com/specta-rs/specta/issues/71
//     assert_zod!(
//         Vec<PlaceholderInnerField>,
//         r#"z.array(z.object({ a: z.string() }))"#
//     );

//     // https://github.com/specta-rs/specta/issues/77
//     assert_eq!(
//         specta_zod::inline::<std::time::SystemTime>(
//             &ExportConfig::new().bigint(BigIntExportBehavior::Number)
//         ),
//         Ok(r#"z.object({ duration_since_epoch: z.number(), duration_since_unix_epoch: z.number() })"#.into())
//     );
//     assert_eq!(
//         specta_zod::inline::<std::time::SystemTime>(
//             &ExportConfig::new().bigint(BigIntExportBehavior::String)
//         ),
//         Ok(r#"z.object({ duration_since_epoch: z.string(), duration_since_unix_epoch: z.number() })"#.into())
//     );

//     assert_eq!(
//         specta_zod::inline::<std::time::Duration>(
//             &ExportConfig::new().bigint(BigIntExportBehavior::Number)
//         ),
//         Ok(r#"z.object({ secs: z.number(), nanos: z.number() })"#.into())
//     );
//     assert_eq!(
//         specta_zod::inline::<std::time::Duration>(
//             &ExportConfig::new().bigint(BigIntExportBehavior::String)
//         ),
//         Ok(r#"z.object({ secs: z.string(), nanos: z.number() })"#.into())
//     );

//     assert_zod!(HashMap<BasicEnum, i32>, r#"z.record(z.union([z.literal("A"), z.literal("B")]), z.number())"#);
//     assert_ts_export!(
//         EnumReferenceRecordKey,
//         "export const EnumReferenceRecordKey = z.object({ a: z.record(BasicEnum, z.number()) })"
//     );

//     assert_zod!(
//         FlattenOnNestedEnum,
//         r#"z.union([z.object({ type: z.literal("a"), value: z.string() }), z.object({ type: z.literal("b"), value: z.number() })]).and(z.object({ id: z.string() }))"#
//     );

//     assert_zod!(PhantomData<()>, r#"z.null()"#);
//     assert_zod!(PhantomData<String>, r#"z.null()"#);
//     assert_zod!(Infallible, r#"z.never()"#);

//     // assert_zod!(Result<String, i32>, r#"z.union([z.string(), z.number()])"#);
//     // assert_zod!(Result<i16, i32>, r#"z.number()"#); // TODO: simplify

//     #[cfg(feature = "either")]
//     {
//         assert_zod!(either::Either<String, i32>, r#"z.union([z.string() z.number()])"#);
//         assert_zod!(either::Either<i16, i32>, r#"z.number()"#);
//     }

//     assert_zod!(Any, r#"z.any()"#);

//     assert_zod!(MyEmptyInput, "z.record(z.string(), z.never())");
//     assert_ts_export!(
//         MyEmptyInput,
//         "export const MyEmptyInput = z.record(z.string(), z.never())"
//     );

//     // https://github.com/specta-rs/specta/issues/142
//     #[allow(unused_parens)]
//     {
//         assert_zod!((String), r#"z.string()"#);
//         assert_zod!((String,), r#"z.tuple([z.string()])"#);
//     }

//     // https://github.com/specta-rs/specta/issues/148
//     assert_zod!(
//         ExtraBracketsInTupleVariant,
//         r#"z.object({ A: z.string() })"#
//     );
//     assert_zod!(ExtraBracketsInUnnamedStruct, "z.string()");

//     // https://github.com/specta-rs/specta/issues/90 // TODO: Fix these
//     // assert_zod!(
//     //     RenameWithWeirdCharsField,
//     //     r#"z.object({ "@odata.context": z.string() })"#
//     // );
//     // assert_zod!(
//     //     RenameWithWeirdCharsVariant,
//     //     r#"z.object({ "@odata.context": z.string() })"#
//     // );
//     // assert_ts_export!(
//     //     error;
//     //     RenameWithWeirdCharsStruct,
//     //     ExportError::InvalidName(
//     //         NamedLocation::Type,
//     //         #[cfg(not(windows))]
//     //         ExportPath::new_unsafe("crates/specta-zod/tests/lib.rs:661:10"),
//     //         #[cfg(windows)]
//     //         ExportPath::new_unsafe("crates\\specta-zod\\tests\\lib.rs:661:10"),
//     //         r#"@odata.context"#.to_string()
//     //     )
//     // );
//     // assert_ts_export!(
//     //     error;
//     //     RenameWithWeirdCharsEnum,
//     //     ExportError::InvalidName(
//     //         NamedLocation::Type,
//     //         #[cfg(not(windows))]
//     //         ExportPath::new_unsafe("crates/specta-zod/tests/lib.rs:665:10"),
//     //         #[cfg(windows)]
//     //         ExportPath::new_unsafe("crates\\specta-zod\\tests\\lib.rs:665:10"),
//     //         r#"@odata.context"#.to_string()
//     //     )
//     // );

//     // https://github.com/specta-rs/specta/issues/156
//     assert_zod!(
//         Vec<MyEnum>,
//         r#"z.array(z.union([z.object({ A: z.string() }), z.object({ B: z.number() })]))"#
//     );

//     assert_zod!(
//         InlineTuple,
//         r#"z.object({ demo: z.tuple([z.string(), z.boolean()]) })"#
//     );
//     assert_zod!(
//         InlineTuple2,
//         r#"z.object({ demo: z.tuple([z.object({ demo: z.tuple([z.string(), z.boolean()]) }), z.boolean()]) })"#
//     );

//     // https://github.com/specta-rs/specta/issues/220
//     // assert_zod!(Box<str>, r#"z.string()"#);
// }

// #[derive(Type)]
// #[specta(collect = false)]
// struct Unit1;

// #[derive(Type)]
// #[specta(collect = false)]
// struct Unit2 {}

// #[derive(Type)]
// #[specta(collect = false)]
// struct Unit3();

// #[derive(Type)]
// #[specta(collect = false)]
// struct Unit4(());

// #[derive(Type)]
// #[specta(collect = false)]
// enum Unit5 {
//     A,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// enum Unit6 {
//     A(),
// }

// #[derive(Type)]
// #[specta(collect = false)]
// enum Unit7 {
//     A {},
// }

// #[derive(Type)]
// #[specta(collect = false)]
// struct SimpleStruct {
//     a: i32,
//     b: String,
//     c: (i32, String, RefCell<i32>),
//     d: Vec<String>,
//     e: Option<String>,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// struct TupleStruct1(i32);

// #[derive(Type)]
// #[specta(collect = false)]
// struct TupleStruct3(i32, bool, String);

// #[derive(Type)]
// #[specta(collect = false)]
// #[specta(rename = "HasBeenRenamed")]
// struct RenamedStruct;

// #[derive(Type)]
// #[specta(collect = false)]
// enum TestEnum {
//     Unit,
//     Single(i32),
//     Multiple(i32, i32),
//     Struct { a: i32 },
// }

// #[derive(Type)]
// #[specta(collect = false)]
// struct RefStruct(TestEnum);

// #[derive(Type)]
// #[specta(collect = false)]
// struct InlineStruct {
//     ref_struct: SimpleStruct,
//     val: i32,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// struct InlinerStruct {
//     #[specta(inline)]
//     inline_this: InlineStruct,
//     dont_inline_this: RefStruct,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// struct GenericStruct<T> {
//     arg: T,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// struct FlattenEnumStruct {
//     outer: String,
//     #[serde(flatten)]
//     inner: FlattenEnum,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// #[serde(tag = "tag", content = "test")]
// enum FlattenEnum {
//     One,
//     Two,
//     Three,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// struct OverridenStruct {
//     #[specta(type = String)]
//     overriden_field: i32,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// struct HasGenericAlias(GenericAlias<i32>);

// type GenericAlias<T> = std::collections::HashMap<T, String>;

// #[derive(Type)]
// #[specta(collect = false)]
// enum SkipVariant {
//     A(String),
//     #[serde(skip)]
//     B(i32),
//     #[specta(skip)]
//     C(i32),
// }

// #[derive(Type)]
// #[specta(collect = false)]
// #[serde(tag = "tag", content = "data")]
// enum SkipVariant2 {
//     A(String),
//     #[serde(skip)]
//     B(i32),
//     #[specta(skip)]
//     C(i32),
// }

// #[derive(Type)]
// #[specta(collect = false)]
// enum SkipVariant3 {
//     A {
//         a: String,
//     },
//     #[serde(skip)]
//     B {
//         b: i32,
//     },
//     #[specta(skip)]
//     C {
//         b: i32,
//     },
// }

// #[derive(Type)]
// #[specta(collect = false)]
// pub enum EnumMacroAttributes {
//     A(#[specta(type = String)] i32),
//     #[specta(rename = "bbb")]
//     B(i32),
//     #[specta(rename = "cccc")]
//     C(#[specta(type = i32)] String),
//     D {
//         #[specta(type = String)]
//         a: i32,
//         #[specta(rename = "bbbbbb")]
//         b: i32,
//     },
// }

// #[derive(Type)]
// #[specta(collect = false)]
// pub struct PlaceholderInnerField {
//     a: String,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// pub struct Recursive {
//     a: i32,
//     children: Vec<Recursive>,
// }

// #[derive(Type)]
// #[specta(collect = false)]

// pub enum InlineEnumField {
//     #[specta(inline)]
//     A(PlaceholderInnerField),
// }

// #[derive(Type)]
// #[specta(collect = false)]
// pub struct InlineOptionalType {
//     #[specta(inline)]
//     pub optional_field: Option<PlaceholderInnerField>,
// }

// const CONTAINER_NAME: &str = "RenameToValueNewName";
// const FIELD_NAME: &str = "demo_new_name";

// // This is very much an advanced API. It is not recommended to use this unless you know what your doing.
// // For personal reference: Is used in PCR to apply an inflection to the dynamic name of the include/select macro.
// #[derive(Type)]
// #[specta(collect = false, rename_from_path = CONTAINER_NAME)]
// pub struct RenameToValue {
//     #[specta(rename_from_path = FIELD_NAME)]
//     pub demo: i32,
// }

// // Regression test for https://github.com/specta-rs/specta/issues/56
// #[derive(Type)]
// #[specta(collect = false)]
// enum Rename {
//     OneWord,
//     #[serde(rename = "Two words")]
//     TwoWords,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// pub struct TransparentTypeInner {
//     inner: String,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// #[serde(transparent)]
// pub struct TransparentType(pub(crate) TransparentTypeInner);

// #[derive(Type)]
// #[specta(collect = false)]
// #[serde(transparent)]
// pub struct TransparentType2(pub(crate) ());

// #[derive()]
// pub struct NonTypeType;

// #[derive(Type)]
// #[specta(collect = false)]
// #[serde(transparent)]
// pub struct TransparentTypeWithOverride(#[specta(type = String)] NonTypeType);

// #[derive(Type)]
// #[specta(collect = false)]
// pub enum BasicEnum {
//     A,
//     B,
// }

// #[derive(Type)]
// #[serde(
//     collect = false,
//     tag = "type",
//     content = "value",
//     rename_all = "camelCase"
// )]
// pub enum NestedEnum {
//     A(String),
//     B(i32),
// }

// #[derive(Type)]
// #[serde(collect = false, rename_all = "camelCase")]
// pub struct FlattenOnNestedEnum {
//     id: String,
//     #[serde(flatten)]
//     result: NestedEnum,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// pub struct EnumReferenceRecordKey {
//     a: HashMap<BasicEnum, i32>,
// }

// // https://github.com/specta-rs/specta/issues/88
// #[derive(Type)]
// #[serde(collect = false, rename_all = "camelCase")]
// #[serde(default)]
// pub struct MyEmptyInput {}

// #[derive(Type)]
// #[specta(collect = false)]
// #[allow(unused_parens)]
// pub enum ExtraBracketsInTupleVariant {
//     A((String)),
// }

// #[derive(Type)]
// #[specta(collect = false)]
// #[allow(unused_parens)]
// pub struct ExtraBracketsInUnnamedStruct((String));

// #[derive(Type)]
// #[specta(collect = false)]
// #[allow(unused_parens)]
// pub struct RenameWithWeirdCharsField {
//     #[specta(rename = "@odata.context")]
//     odata_context: String,
// }

// #[derive(Type)]
// #[specta(collect = false)]
// #[allow(unused_parens)]
// pub enum RenameWithWeirdCharsVariant {
//     #[specta(rename = "@odata.context")]
//     A(String),
// }

// #[derive(Type)]
// #[specta(collect = false, rename = "@odata.context")]
// pub struct RenameWithWeirdCharsStruct(String);

// #[derive(Type)]
// #[specta(collect = false, rename = "@odata.context")]
// pub enum RenameWithWeirdCharsEnum {}

// #[derive(Type)]
// pub enum MyEnum {
//     A(String),
//     B(u32),
// }

// #[derive(Type)]
// pub struct InlineTuple {
//     #[specta(inline)]
//     demo: (String, bool),
// }

// #[derive(Type)]
// pub struct InlineTuple2 {
//     #[specta(inline)]
//     demo: (InlineTuple, bool),
// }
