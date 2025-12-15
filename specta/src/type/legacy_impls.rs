// //! The plan is to try and move these into the ecosystem for the v2 release.
// use super::macros::*;
// use crate::{datatype::*, Flatten, Type, TypeCollection};

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
//         fn definition(_: &mut TypeCollection) -> DataType {
//             DataType::Enum(Enum {
//                 repr: Some(EnumRepr::Untagged),
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
//                                     inline: false,
//                                     deprecated: None,
//                                     docs: Cow::Borrowed(""),
//                                     ty: Some(DataType::Primitive(Primitive::f64)),
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
//                                     inline: false,
//                                     deprecated: None,
//                                     docs: Cow::Borrowed(""),
//                                     ty: Some(DataType::Primitive(Primitive::i64)),
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
//                                     inline: false,
//                                     deprecated: None,
//                                     docs: Cow::Borrowed(""),
//                                     ty: Some(DataType::Primitive(Primitive::u64)),
//                                 }],
//                             }),
//                         },
//                     ),
//                 ],
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
//         fn definition(types: &mut TypeCollection) -> DataType {
//             // We don't type this more accurately because `serde_json` doesn't allow non-string map keys so neither does Specta // TODO
//             std::collections::HashMap::<serde_yaml::Value, serde_yaml::Value>::definition(types)
//         }
//     }

//     impl Type for serde_yaml::value::TaggedValue {
//         fn definition(types: &mut TypeCollection) -> DataType {
//             std::collections::HashMap::<String, serde_yaml::Value>::definition(types)
//         }
//     }

//     impl Type for serde_yaml::Number {
//         fn definition(_: &mut TypeCollection) -> DataType {
//             DataType::Enum(Enum {
//                 repr: Some(EnumRepr::Untagged),
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
//                                     inline: false,
//                                     deprecated: None,
//                                     docs: Cow::Borrowed(""),
//                                     ty: Some(DataType::Primitive(Primitive::f64)),
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
//                                     inline: false,
//                                     deprecated: None,
//                                     docs: Cow::Borrowed(""),
//                                     ty: Some(DataType::Primitive(Primitive::i64)),
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
//                                     inline: false,
//                                     deprecated: None,
//                                     docs: Cow::Borrowed(""),
//                                     ty: Some(DataType::Primitive(Primitive::u64)),
//                                 }],
//                             }),
//                         },
//                     ),
//                 ],
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
//     #[specta(rename = "TomlValue", untagged, remote = Value, crate = crate, export = false)]
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
//     macro_rules! implement_specta_type_for_glam_type {
//         (
//             $name: ident as $representation: ty
//         ) => {
//             #[derive(Type)]
//             #[specta(remote = glam::$name, crate = crate, export = false)]
//             #[allow(dead_code)]
//             struct $name($representation);
//         };
//     }

//     // Implementations for https://docs.rs/glam/latest/glam/f32/index.html
//     // Affines
//     implement_specta_type_for_glam_type!(Affine2 as [f32; 6]);
//     implement_specta_type_for_glam_type!(Affine3A as [f32; 12]);

//     // Matrices
//     implement_specta_type_for_glam_type!(Mat2 as [f32; 4]);
//     implement_specta_type_for_glam_type!(Mat3 as [f32; 9]);
//     implement_specta_type_for_glam_type!(Mat3A as [f32; 9]);
//     implement_specta_type_for_glam_type!(Mat4 as [f32; 16]);

//     // Quaternions
//     implement_specta_type_for_glam_type!(Quat as [f32; 4]);

//     // Vectors
//     implement_specta_type_for_glam_type!(Vec2 as [f32; 2]);
//     implement_specta_type_for_glam_type!(Vec3 as [f32; 3]);
//     implement_specta_type_for_glam_type!(Vec3A as [f32; 3]);
//     implement_specta_type_for_glam_type!(Vec4 as [f32; 4]);

//     // Implementations for https://docs.rs/glam/latest/glam/f64/index.html
//     // Affines
//     implement_specta_type_for_glam_type!(DAffine2 as [f64; 6]);
//     implement_specta_type_for_glam_type!(DAffine3 as [f64; 12]);

//     // Matrices
//     implement_specta_type_for_glam_type!(DMat2 as [f64; 4]);
//     implement_specta_type_for_glam_type!(DMat3 as [f64; 9]);
//     implement_specta_type_for_glam_type!(DMat4 as [f64; 16]);

//     // Quaternions
//     implement_specta_type_for_glam_type!(DQuat as [f64; 4]);

//     // Vectors
//     implement_specta_type_for_glam_type!(DVec2 as [f64; 2]);
//     implement_specta_type_for_glam_type!(DVec3 as [f64; 3]);
//     implement_specta_type_for_glam_type!(DVec4 as [f64; 4]);

//     // Implementations for https://docs.rs/glam/latest/glam/i8/index.html
//     implement_specta_type_for_glam_type!(I8Vec2 as [i8; 2]);
//     implement_specta_type_for_glam_type!(I8Vec3 as [i8; 3]);
//     implement_specta_type_for_glam_type!(I8Vec4 as [i8; 4]);

//     // Implementations for https://docs.rs/glam/latest/glam/u8/index.html
//     implement_specta_type_for_glam_type!(U8Vec2 as [u8; 2]);
//     implement_specta_type_for_glam_type!(U8Vec3 as [u8; 3]);
//     implement_specta_type_for_glam_type!(U8Vec4 as [u8; 4]);

//     // Implementations for https://docs.rs/glam/latest/glam/i16/index.html
//     implement_specta_type_for_glam_type!(I16Vec2 as [i16; 2]);
//     implement_specta_type_for_glam_type!(I16Vec3 as [i16; 3]);
//     implement_specta_type_for_glam_type!(I16Vec4 as [i16; 4]);

//     // Implementations for https://docs.rs/glam/latest/glam/u16/index.html
//     implement_specta_type_for_glam_type!(U16Vec2 as [u16; 2]);
//     implement_specta_type_for_glam_type!(U16Vec3 as [u16; 3]);
//     implement_specta_type_for_glam_type!(U16Vec4 as [u16; 4]);

//     // Implementations for https://docs.rs/glam/latest/glam/i32/index.html
//     implement_specta_type_for_glam_type!(IVec2 as [i32; 2]);
//     implement_specta_type_for_glam_type!(IVec3 as [i32; 3]);
//     implement_specta_type_for_glam_type!(IVec4 as [i32; 4]);

//     // Implementations for https://docs.rs/glam/latest/glam/u32/index.html
//     implement_specta_type_for_glam_type!(UVec2 as [u32; 2]);
//     implement_specta_type_for_glam_type!(UVec3 as [u32; 3]);
//     implement_specta_type_for_glam_type!(UVec4 as [u32; 4]);

//     // Implementation for https://docs.rs/glam/latest/glam/i64/index.html
//     implement_specta_type_for_glam_type!(I64Vec2 as [i64; 2]);
//     implement_specta_type_for_glam_type!(I64Vec3 as [i64; 3]);
//     implement_specta_type_for_glam_type!(I64Vec4 as [i64; 4]);

//     // Implementation for https://docs.rs/glam/latest/glam/u64/index.html
//     implement_specta_type_for_glam_type!(U64Vec2 as [u64; 2]);
//     implement_specta_type_for_glam_type!(U64Vec3 as [u64; 3]);
//     implement_specta_type_for_glam_type!(U64Vec4 as [u64; 4]);

//     // implementation for https://docs.rs/glam/latest/glam/usize/index.html
//     implement_specta_type_for_glam_type!(USizeVec2 as [usize; 2]);
//     implement_specta_type_for_glam_type!(USizeVec3 as [usize; 3]);
//     implement_specta_type_for_glam_type!(USizeVec4 as [usize; 4]);

//     // Implementation for https://docs.rs/glam/latest/glam/bool/index.html
//     implement_specta_type_for_glam_type!(BVec2 as [bool; 2]);
//     implement_specta_type_for_glam_type!(BVec3 as [bool; 3]);
//     implement_specta_type_for_glam_type!(BVec4 as [bool; 4]);
// };

// #[cfg(feature = "url")]
// impl_as!(url::Url as String);

// #[cfg(feature = "either")]
// impl<L: Type, R: Type> Type for either::Either<L, R> {
//     fn definition(types: &mut TypeCollection) -> DataType {
//         DataType::Enum(Enum {
//             repr: Some(EnumRepr::Untagged),
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
//                                 inline: false,
//                                 deprecated: None,
//                                 docs: Cow::Borrowed(""),
//                                 ty: Some(L::definition(types)),
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
//                                 inline: false,
//                                 deprecated: None,
//                                 docs: Cow::Borrowed(""),
//                                 ty: Some(R::definition(types)),
//                             }],
//                         }),
//                     },
//                 ),
//             ],
//         })
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

// #[cfg(feature = "camino")]
// impl_as!(
//     camino::Utf8Path as String
//     camino::Utf8PathBuf as String
// );
