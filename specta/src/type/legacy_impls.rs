// //! TODO: These are all planned to be removed from the core and into the ecosystem for the v2 release.
// use crate::{datatype::reference::Reference, datatype::*, r#type::macros::*, *};

// use std::borrow::Cow;

// #[cfg(feature = "indexmap")]
// const _: () = {
//     impl_for_list!(true; indexmap::IndexSet<T> as "IndexSet");
//     impl_for_map!(indexmap::IndexMap<K, V> as "IndexMap");
//     impl<K: Type, V: Type> Flatten for indexmap::IndexMap<K, V> {}
// };

// #[cfg(feature = "serde_json")]
// const _: () = {
//     use serde_json::{Map, Number, Value};

//     impl_for_map!(Map<K, V> as "Map");
//     impl<K: Type, V: Type> Flatten for Map<K, V> {}

//     #[derive(Type)]
//     #[specta(rename = "JsonValue", untagged, remote = Value, crate = crate, export = false)]
//     pub enum JsonValue {
//         Null,
//         Bool(bool),
//         Number(Number),
//         String(String),
//         Array(Vec<Value>),
//         Object(Map<String, Value>),
//     }

//     impl Type for Number {
//         fn inline(_: &mut TypeCollection, _: Generics) -> DataType {
//             DataType::Enum(EnumType {
//                 name: "Number".into(),
//                 sid: None,
//                 repr: EnumRepr::Untagged,
//                 skip_bigint_checks: true,
//                 variants: vec![
//                     (
//                         "f64".into(),
//                         EnumVariant {
//                             skip: false,
//                             docs: Cow::Borrowed(""),
//                             deprecated: None,
//                             fields: Fields::Unnamed(UnnamedFields {
//                                 fields: vec![Field {
//                                     optional: false,
//                                     flatten: false,
//                                     deprecated: None,
//                                     docs: Cow::Borrowed(""),
//                                     ty: Some(DataType::Primitive(PrimitiveType::f64)),
//                                 }],
//                             }),
//                         },
//                     ),
//                     (
//                         "i64".into(),
//                         EnumVariant {
//                             skip: false,
//                             docs: Cow::Borrowed(""),
//                             deprecated: None,
//                             fields: Fields::Unnamed(UnnamedFields {
//                                 fields: vec![Field {
//                                     optional: false,
//                                     flatten: false,
//                                     deprecated: None,
//                                     docs: Cow::Borrowed(""),
//                                     ty: Some(DataType::Primitive(PrimitiveType::i64)),
//                                 }],
//                             }),
//                         },
//                     ),
//                     (
//                         "u64".into(),
//                         EnumVariant {
//                             skip: false,
//                             docs: Cow::Borrowed(""),
//                             deprecated: None,
//                             fields: Fields::Unnamed(UnnamedFields {
//                                 fields: vec![Field {
//                                     optional: false,
//                                     flatten: false,
//                                     deprecated: None,
//                                     docs: Cow::Borrowed(""),
//                                     ty: Some(DataType::Primitive(PrimitiveType::u64)),
//                                 }],
//                             }),
//                         },
//                     ),
//                 ],
//                 generics: vec![],
//             })
//         }
//     }
// };

// #[cfg(feature = "serde_yaml")]
// const _: () = {
//     use serde_yaml::{value::TaggedValue, Mapping, Number, Sequence, Value};

//     #[derive(Type)]
//     #[specta(rename = "YamlValue", untagged, remote = Value, crate = crate, export = false)]
//     pub enum YamlValue {
//         Null,
//         Bool(bool),
//         Number(Number),
//         String(String),
//         Sequence(Sequence),
//         Mapping(Mapping),
//         Tagged(Box<TaggedValue>),
//     }

//     impl Type for serde_yaml::Mapping {
//         fn inline(_: &mut TypeCollection, _: Generics) -> DataType {
//             // We don't type this more accurately because `serde_json` doesn't allow non-string map keys so neither does Specta
//             DataType::Unknown
//         }
//     }

//     impl Type for serde_yaml::value::TaggedValue {
//         fn inline(_: &mut TypeCollection, _: Generics) -> DataType {
//             DataType::Map(Map {
//                 key_ty: Box::new(DataType::Primitive(PrimitiveType::String)),
//                 value_ty: Box::new(DataType::Unknown),
//             })
//         }
//     }

//     impl Type for serde_yaml::Number {
//         fn inline(_: &mut TypeCollection, _: Generics) -> DataType {
//             DataType::Enum(EnumType {
//                 name: "Number".into(),
//                 sid: None,
//                 repr: EnumRepr::Untagged,
//                 skip_bigint_checks: true,
//                 variants: vec![
//                     (
//                         "f64".into(),
//                         EnumVariant {
//                             skip: false,
//                             docs: Cow::Borrowed(""),
//                             deprecated: None,
//                             fields: Fields::Unnamed(UnnamedFields {
//                                 fields: vec![Field {
//                                     optional: false,
//                                     flatten: false,
//                                     deprecated: None,
//                                     docs: Cow::Borrowed(""),
//                                     ty: Some(DataType::Primitive(PrimitiveType::f64)),
//                                 }],
//                             }),
//                         },
//                     ),
//                     (
//                         "i64".into(),
//                         EnumVariant {
//                             skip: false,
//                             docs: Cow::Borrowed(""),
//                             deprecated: None,
//                             fields: Fields::Unnamed(UnnamedFields {
//                                 fields: vec![Field {
//                                     optional: false,
//                                     flatten: false,
//                                     deprecated: None,
//                                     docs: Cow::Borrowed(""),
//                                     ty: Some(DataType::Primitive(PrimitiveType::i64)),
//                                 }],
//                             }),
//                         },
//                     ),
//                     (
//                         "u64".into(),
//                         EnumVariant {
//                             skip: false,
//                             docs: Cow::Borrowed(""),
//                             deprecated: None,
//                             fields: Fields::Unnamed(UnnamedFields {
//                                 fields: vec![Field {
//                                     optional: false,
//                                     flatten: false,
//                                     deprecated: None,
//                                     docs: Cow::Borrowed(""),
//                                     ty: Some(DataType::Primitive(PrimitiveType::u64)),
//                                 }],
//                             }),
//                         },
//                     ),
//                 ],
//                 generics: vec![],
//             })
//         }
//     }
// };

// #[cfg(feature = "toml")]
// const _: () = {
//     use toml::{value::Array, value::Datetime, value::Table, Value};

//     impl_for_map!(toml::map::Map<K, V> as "Map");
//     impl<K: Type, V: Type> Flatten for toml::map::Map<K, V> {}

//     #[derive(Type)]
//     #[specta(rename = "TomlValue", untagged, remote = Value, crate = crate, export = false, unstable_skip_bigint_checks)]
//     pub enum TomlValue {
//         String(String),
//         Integer(i64),
//         Float(f64),
//         Boolean(bool),
//         Datetime(Datetime),
//         Array(Array),
//         Table(Table),
//     }

//     #[derive(Type)]
//     #[specta(rename = "Datetime", remote = Datetime, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct DatetimeDef {
//         #[specta(rename = "$__toml_private_datetime")]
//         pub v: String,
//     }
// };

// #[cfg(feature = "ulid")]
// impl_as!(ulid::Ulid as String);

// #[cfg(feature = "uuid")]
// impl_as!(
//     uuid::Uuid as String
//     uuid::fmt::Hyphenated as String
// );

// #[cfg(feature = "chrono")]
// const _: () = {
//     use chrono::*;

//     impl_as!(
//         NaiveDateTime as String
//         NaiveDate as String
//         NaiveTime as String
//         chrono::Duration as String
//     );

//     impl<T: TimeZone> Type for DateTime<T> {
//         impl_passthrough!(String);
//     }

//     #[allow(deprecated)]
//     impl<T: TimeZone> Type for Date<T> {
//         impl_passthrough!(String);
//     }
// };

// #[cfg(feature = "time")]
// impl_as!(
//     time::PrimitiveDateTime as String
//     time::OffsetDateTime as String
//     time::Date as String
//     time::Time as String
//     time::Duration as String
//     time::Weekday as String
// );

// #[cfg(feature = "jiff")]
// impl_as!(
//     jiff::Timestamp as String
//     jiff::Zoned as String
//     jiff::Span as String
//     jiff::civil::Date as String
//     jiff::civil::Time as String
//     jiff::civil::DateTime as String
//     jiff::tz::TimeZone as String
// );

// #[cfg(feature = "bigdecimal")]
// impl_as!(bigdecimal::BigDecimal as String);

// // This assumes the `serde-with-str` feature is enabled. Check #26 for more info.
// #[cfg(feature = "rust_decimal")]
// impl_as!(rust_decimal::Decimal as String);

// #[cfg(feature = "ipnetwork")]
// impl_as!(
//     ipnetwork::IpNetwork as String
//     ipnetwork::Ipv4Network as String
//     ipnetwork::Ipv6Network as String
// );

// #[cfg(feature = "mac_address")]
// impl_as!(mac_address::MacAddress as String);

// #[cfg(feature = "chrono")]
// impl_as!(
//     chrono::FixedOffset as String
//     chrono::Utc as String
//     chrono::Local as String
// );

// #[cfg(feature = "bson")]
// impl_as!(
//     bson::oid::ObjectId as String
//     bson::Decimal128 as i128
//     bson::DateTime as String
//     bson::Uuid as String
// );

// // TODO: bson::bson
// // TODO: bson::Document

// #[cfg(feature = "bytesize")]
// impl_as!(bytesize::ByteSize as u64);

// #[cfg(feature = "uhlc")]
// const _: () = {
//     use std::num::NonZeroU128;

//     use uhlc::*;

//     impl_as!(
//         NTP64 as u64
//         ID as NonZeroU128
//     );

//     #[derive(Type)]
//     #[specta(remote = Timestamp, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct Timestamp {
//         time: NTP64,
//         id: ID,
//     }
// };

// #[cfg(feature = "glam")]
// const _: () = {
//     #[derive(Type)]
//     #[specta(remote = glam::DVec2, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct DVec2([f64; 2]);

//     #[derive(Type)]
//     #[specta(remote = glam::IVec2, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct IVec2([i32; 2]);

//     #[derive(Type)]
//     #[specta(remote = glam::DMat2, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct DMat2([f64; 4]);

//     #[derive(Type)]
//     #[specta(remote = glam::DAffine2, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct DAffine2([f64; 6]);

//     #[derive(Type)]
//     #[specta(remote = glam::Vec2, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct Vec2([f32; 2]);

//     #[derive(Type)]
//     #[specta(remote = glam::Vec3, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct Vec3([f32; 3]);

//     #[derive(Type)]
//     #[specta(remote = glam::Vec3A, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct Vec3A([f32; 3]);

//     #[derive(Type)]
//     #[specta(remote = glam::Vec4, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct Vec4([f32; 4]);

//     #[derive(Type)]
//     #[specta(remote = glam::Mat2, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct Mat2([f32; 4]);

//     #[derive(Type)]
//     #[specta(remote = glam::Mat3, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct Mat3([f32; 9]);

//     #[derive(Type)]
//     #[specta(remote = glam::Mat3A, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct Mat3A([f32; 9]);

//     #[derive(Type)]
//     #[specta(remote = glam::Mat4, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct Mat4([f32; 16]);

//     #[derive(Type)]
//     #[specta(remote = glam::Quat, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct Quat([f32; 4]);

//     #[derive(Type)]
//     #[specta(remote = glam::Affine2, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct Affine2([f32; 6]);

//     #[derive(Type)]
//     #[specta(remote = glam::Affine3A, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct Affine3A([f32; 12]);
// };

// #[cfg(feature = "url")]
// impl_as!(url::Url as String);

// #[cfg(feature = "either")]
// impl<L: Type, R: Type> Type for either::Either<L, R> {
//     fn inline(types: &mut TypeCollection, generics: Generics) -> DataType {
//         DataType::Enum(EnumType {
//             name: "Either".into(),
//             sid: None,
//             repr: EnumRepr::Untagged,
//             skip_bigint_checks: false,
//             variants: vec![
//                 (
//                     "Left".into(),
//                     EnumVariant {
//                         skip: false,
//                         docs: Cow::Borrowed(""),
//                         deprecated: None,
//                         fields: Fields::Unnamed(UnnamedFields {
//                             fields: vec![Field {
//                                 optional: false,
//                                 flatten: false,
//                                 deprecated: None,
//                                 docs: Cow::Borrowed(""),
//                                 ty: Some(L::inline(types, generics)),
//                             }],
//                         }),
//                     },
//                 ),
//                 (
//                     "Right".into(),
//                     EnumVariant {
//                         skip: false,
//                         docs: Cow::Borrowed(""),
//                         deprecated: None,
//                         fields: Fields::Unnamed(UnnamedFields {
//                             fields: vec![Field {
//                                 optional: false,
//                                 flatten: false,
//                                 deprecated: None,
//                                 docs: Cow::Borrowed(""),
//                                 ty: Some(R::inline(types, generics)),
//                             }],
//                         }),
//                     },
//                 ),
//             ],
//             generics: vec![],
//         })
//     }

//     fn reference(types: &mut TypeCollection, generics: &[DataType]) -> Option<Reference> {
//         // Reference {
//         //     inner: DataType::Enum(EnumType {
//         //         name: "Either".into(),
//         //         sid: None,
//         //         repr: EnumRepr::Untagged,
//         //         skip_bigint_checks: false,
//         //         variants: vec![
//         //             (
//         //                 "Left".into(),
//         //                 EnumVariant {
//         //                     skip: false,
//         //                     docs: Cow::Borrowed(""),
//         //                     deprecated: None,
//         //                     fields: Fields::Unnamed(UnnamedFields {
//         //                         fields: vec![Field {
//         //                             optional: false,
//         //                             flatten: false,
//         //                             deprecated: None,
//         //                             docs: Cow::Borrowed(""),
//         //                             ty: Some(L::reference(types, generics).inner),
//         //                         }],
//         //                     }),
//         //                 },
//         //             ),
//         //             (
//         //                 "Right".into(),
//         //                 EnumVariant {
//         //                     skip: false,
//         //                     docs: Cow::Borrowed(""),
//         //                     deprecated: None,
//         //                     fields: Fields::Unnamed(UnnamedFields {
//         //                         fields: vec![Field {
//         //                             optional: false,
//         //                             flatten: false,
//         //                             deprecated: None,
//         //                             docs: Cow::Borrowed(""),
//         //                             ty: Some(R::reference(types, generics).inner),
//         //                         }],
//         //                     }),
//         //                 },
//         //             ),
//         //         ],
//         //         generics: vec![],
//         //     }),
//         // }
//         todo!();
//     }
// }

// #[cfg(feature = "bevy_ecs")]
// const _: () = {
//     #[derive(Type)]
//     #[specta(rename = "Entity", remote = bevy_ecs::entity::Entity, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct EntityDef(u64);
// };

// #[cfg(feature = "bevy_input")]
// const _: () = {
//     #[derive(Type)]
//     #[specta(remote = bevy_input::ButtonState, crate = crate, export = false)]
//     #[allow(dead_code)]
//     enum ButtonState {
//         Pressed,
//         Released,
//     }

//     #[derive(Type)]
//     #[specta(remote = bevy_input::keyboard::KeyboardInput, crate = crate, export = false)]
//     #[allow(dead_code)]
//     struct KeyboardInput {
//         pub key_code: bevy_input::keyboard::KeyCode,
//         pub logical_key: bevy_input::keyboard::Key,
//         pub state: bevy_input::ButtonState,
//         pub window: bevy_ecs::entity::Entity,
//     }

//     // Reduced KeyCode and Key to String to avoid redefining a quite large enum (for now)
//     impl_as!(
//         bevy_input::keyboard::KeyCode as String
//         bevy_input::keyboard::Key as String
//     );

//     #[derive(Type)]
//     #[specta(remote = bevy_input::mouse::MouseButtonInput, crate = crate, export = false)]
//     #[allow(dead_code)]
//     pub struct MouseButtonInput {
//         pub button: bevy_input::mouse::MouseButton,
//         pub state: bevy_input::ButtonState,
//         pub window: bevy_ecs::entity::Entity,
//     }

//     #[derive(Type)]
//     #[specta(remote = bevy_input::mouse::MouseButton, crate = crate, export = false)]
//     #[allow(dead_code)]
//     pub enum MouseButton {
//         Left,
//         Right,
//         Middle,
//         Back,
//         Forward,
//         Other(u16),
//     }

//     #[derive(Type)]
//     #[specta(remote = bevy_input::mouse::MouseWheel, crate = crate, export = false)]
//     #[allow(dead_code)]
//     pub struct MouseWheel {
//         pub unit: bevy_input::mouse::MouseScrollUnit,
//         pub x: f32,
//         pub y: f32,
//         pub window: bevy_ecs::entity::Entity,
//     }

//     #[derive(Type)]
//     #[specta(remote = bevy_input::mouse::MouseScrollUnit, crate = crate, export = false)]
//     #[allow(dead_code)]
//     pub enum MouseScrollUnit {
//         Line,
//         Pixel,
//     }

//     #[derive(Type)]
//     #[specta(remote = bevy_input::mouse::MouseMotion, crate = crate, export = false)]
//     #[allow(dead_code)]
//     pub struct MouseMotion {
//         pub delta: glam::Vec2,
//     }
// };
