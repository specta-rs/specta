//! The plan is to try and move these into the ecosystem for the v2 release.
use super::macros::*;
use crate::{Type, TypeCollection, datatype::*};

use std::borrow::Cow;

#[cfg(feature = "indexmap")]
const _: () = {
    impl_for_list!(true; indexmap::IndexSet<T>);
    impl_for_map!(indexmap::IndexMap<K, V>);
};

#[cfg(feature = "serde_json")]
const _: () = {
    use serde_json::{Map, Number, Value};

    impl_for_map!(Map<K, V>);

    impl Type for Value {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    ("Null".into(), EnumVariant::unit()),
                    (
                        "Bool".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(bool::definition(types)))
                            .build(),
                    ),
                    (
                        "Number".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(Number::definition(types)))
                            .build(),
                    ),
                    (
                        "String".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(String::definition(types)))
                            .build(),
                    ),
                    (
                        "Array".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(Vec::<Value>::definition(types)))
                            .build(),
                    ),
                    (
                        "Object".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(Map::<String, Value>::definition(types)))
                            .build(),
                    ),
                ],
                attributes: vec![],
            })
        }
    }

    impl Type for Number {
        fn definition(_: &mut TypeCollection) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    (
                        "f64".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(DataType::Primitive(Primitive::f64)))
                            .build(),
                    ),
                    (
                        "i64".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(DataType::Primitive(Primitive::i64)))
                            .build(),
                    ),
                    (
                        "u64".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(DataType::Primitive(Primitive::u64)))
                            .build(),
                    ),
                ],
                attributes: vec![RuntimeAttribute {
                    path: String::from("serde"),
                    kind: RuntimeMeta::List(vec![RuntimeNestedMeta::Meta(RuntimeMeta::Path(
                        String::from("untagged"),
                    ))]),
                }],
            })
        }
    }
};

#[cfg(feature = "serde_yaml")]
const _: () = {
    use serde_yaml::{Number, Value, value::TaggedValue};

    impl Type for serde_yaml::Mapping {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Map(crate::datatype::Map::new(
                serde_yaml::Value::definition(types),
                serde_yaml::Value::definition(types),
            ))
        }
    }

    impl Type for serde_yaml::value::TaggedValue {
        fn definition(types: &mut TypeCollection) -> DataType {
            std::collections::HashMap::<String, serde_yaml::Value>::definition(types)
        }
    }

    impl Type for Value {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    ("Null".into(), EnumVariant::unit()),
                    (
                        "Bool".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(bool::definition(types)))
                            .build(),
                    ),
                    (
                        "Number".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(Number::definition(types)))
                            .build(),
                    ),
                    (
                        "String".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(String::definition(types)))
                            .build(),
                    ),
                    (
                        "Sequence".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(Vec::<Value>::definition(types)))
                            .build(),
                    ),
                    (
                        "Mapping".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(std::collections::BTreeMap::<
                                serde_yaml::Value,
                                serde_yaml::Value,
                            >::definition(types)))
                            .build(),
                    ),
                    (
                        "Tagged".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(Box::<TaggedValue>::definition(types)))
                            .build(),
                    ),
                ],
                attributes: vec![],
            })
        }
    }

    impl Type for serde_yaml::Number {
        fn definition(_: &mut TypeCollection) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    (
                        "f64".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(DataType::Primitive(Primitive::f64)))
                            .build(),
                    ),
                    (
                        "i64".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(DataType::Primitive(Primitive::i64)))
                            .build(),
                    ),
                    (
                        "u64".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(DataType::Primitive(Primitive::u64)))
                            .build(),
                    ),
                ],
                attributes: vec![],
            })
        }
    }
};

#[cfg(feature = "toml")]
const _: () = {
    use toml::{Value, value};

    impl_for_map!(toml::map::Map<K, V>);

    impl Type for value::Datetime {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Struct(Struct {
                fields: Fields::Named(NamedFields {
                    fields: vec![(
                        "v".into(),
                        Field {
                            optional: false,

                            inline: false,
                            deprecated: None,
                            docs: Cow::Borrowed(""),
                            ty: Some(String::definition(types)),
                            attributes: Vec::new(),
                        },
                    )],
                    attributes: Vec::new(),
                }),
                attributes: Vec::new(),
            })
        }
    }

    impl Type for Value {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    (
                        "String".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(String::definition(types)))
                            .build(),
                    ),
                    (
                        "Integer".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(i64::definition(types)))
                            .build(),
                    ),
                    (
                        "Float".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(f64::definition(types)))
                            .build(),
                    ),
                    (
                        "Boolean".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(bool::definition(types)))
                            .build(),
                    ),
                    (
                        "Datetime".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(value::Datetime::definition(types)))
                            .build(),
                    ),
                    (
                        "Array".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(Vec::<Value>::definition(types)))
                            .build(),
                    ),
                    (
                        "Table".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(
                                std::collections::BTreeMap::<String, Value>::definition(types),
                            ))
                            .build(),
                    ),
                ],
                attributes: vec![],
            })
        }
    }
};

#[cfg(feature = "ulid")]
impl_as!(ulid::Ulid as String);

#[cfg(feature = "uuid")]
impl_as!(
    uuid::Uuid as String
    uuid::fmt::Hyphenated as String
);

#[cfg(feature = "chrono")]
const _: () = {
    use chrono::*;

    impl_as!(
        NaiveDateTime as String
        NaiveDate as String
        NaiveTime as String
        chrono::Duration as String
    );

    impl<T: TimeZone> Type for DateTime<T> {
        impl_passthrough!(String);
    }

    #[allow(deprecated)]
    impl<T: TimeZone> Type for Date<T> {
        impl_passthrough!(String);
    }
};

#[cfg(feature = "time")]
impl_as!(
    time::PrimitiveDateTime as String
    time::OffsetDateTime as String
    time::Date as String
    time::Time as String
    time::Duration as String
    time::Weekday as String
);

#[cfg(feature = "jiff")]
impl_as!(
    jiff::Timestamp as String
    jiff::Zoned as String
    jiff::Span as String
    jiff::civil::Date as String
    jiff::civil::Time as String
    jiff::civil::DateTime as String
    jiff::tz::TimeZone as String
);

#[cfg(feature = "bigdecimal")]
impl_as!(bigdecimal::BigDecimal as String);

// This assumes the `serde-with-str` feature is enabled. Check #26 for more info.
#[cfg(feature = "rust_decimal")]
impl_as!(rust_decimal::Decimal as String);

#[cfg(feature = "ipnetwork")]
impl_as!(
    ipnetwork::IpNetwork as String
    ipnetwork::Ipv4Network as String
    ipnetwork::Ipv6Network as String
);

#[cfg(feature = "mac_address")]
impl_as!(mac_address::MacAddress as String);

#[cfg(feature = "chrono")]
impl_as!(
    chrono::FixedOffset as String
    chrono::Utc as String
    chrono::Local as String
);

#[cfg(feature = "bson")]
impl_as!(
    bson::oid::ObjectId as String
    bson::Decimal128 as i128
    bson::DateTime as String
    bson::Uuid as String
);

// TODO: bson::bson
// TODO: bson::Document

#[cfg(feature = "bytesize")]
impl_as!(bytesize::ByteSize as u64);

#[cfg(feature = "uhlc")]
const _: () = {
    use std::num::NonZeroU128;

    use uhlc::*;

    impl_as!(
        NTP64 as u64
        ID as NonZeroU128
    );

    impl Type for Timestamp {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Struct(Struct {
                fields: Fields::Named(NamedFields {
                    fields: vec![
                        (
                            "time".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(NTP64::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                        (
                            "id".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(ID::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                    ],
                    attributes: Vec::new(),
                }),
                attributes: Vec::new(),
            })
        }
    }
};

#[cfg(feature = "glam")]
const _: () = {
    macro_rules! implement_specta_type_for_glam_type {
        (
            $name: ident as $representation: ty
        ) => {
            impl Type for glam::$name {
                fn definition(types: &mut TypeCollection) -> DataType {
                    <$representation>::definition(types)
                }
            }
        };
    }

    // Implementations for https://docs.rs/glam/latest/glam/f32/index.html
    // Affines
    implement_specta_type_for_glam_type!(Affine2 as [f32; 6]);
    implement_specta_type_for_glam_type!(Affine3A as [f32; 12]);

    // Matrices
    implement_specta_type_for_glam_type!(Mat2 as [f32; 4]);
    implement_specta_type_for_glam_type!(Mat3 as [f32; 9]);
    implement_specta_type_for_glam_type!(Mat3A as [f32; 9]);
    implement_specta_type_for_glam_type!(Mat4 as [f32; 16]);

    // Quaternions
    implement_specta_type_for_glam_type!(Quat as [f32; 4]);

    // Vectors
    implement_specta_type_for_glam_type!(Vec2 as [f32; 2]);
    implement_specta_type_for_glam_type!(Vec3 as [f32; 3]);
    implement_specta_type_for_glam_type!(Vec3A as [f32; 3]);
    implement_specta_type_for_glam_type!(Vec4 as [f32; 4]);

    // Implementations for https://docs.rs/glam/latest/glam/f64/index.html
    // Affines
    implement_specta_type_for_glam_type!(DAffine2 as [f64; 6]);
    implement_specta_type_for_glam_type!(DAffine3 as [f64; 12]);

    // Matrices
    implement_specta_type_for_glam_type!(DMat2 as [f64; 4]);
    implement_specta_type_for_glam_type!(DMat3 as [f64; 9]);
    implement_specta_type_for_glam_type!(DMat4 as [f64; 16]);

    // Quaternions
    implement_specta_type_for_glam_type!(DQuat as [f64; 4]);

    // Vectors
    implement_specta_type_for_glam_type!(DVec2 as [f64; 2]);
    implement_specta_type_for_glam_type!(DVec3 as [f64; 3]);
    implement_specta_type_for_glam_type!(DVec4 as [f64; 4]);

    // Implementations for https://docs.rs/glam/latest/glam/i8/index.html
    implement_specta_type_for_glam_type!(I8Vec2 as [i8; 2]);
    implement_specta_type_for_glam_type!(I8Vec3 as [i8; 3]);
    implement_specta_type_for_glam_type!(I8Vec4 as [i8; 4]);

    // Implementations for https://docs.rs/glam/latest/glam/u8/index.html
    implement_specta_type_for_glam_type!(U8Vec2 as [u8; 2]);
    implement_specta_type_for_glam_type!(U8Vec3 as [u8; 3]);
    implement_specta_type_for_glam_type!(U8Vec4 as [u8; 4]);

    // Implementations for https://docs.rs/glam/latest/glam/i16/index.html
    implement_specta_type_for_glam_type!(I16Vec2 as [i16; 2]);
    implement_specta_type_for_glam_type!(I16Vec3 as [i16; 3]);
    implement_specta_type_for_glam_type!(I16Vec4 as [i16; 4]);

    // Implementations for https://docs.rs/glam/latest/glam/u16/index.html
    implement_specta_type_for_glam_type!(U16Vec2 as [u16; 2]);
    implement_specta_type_for_glam_type!(U16Vec3 as [u16; 3]);
    implement_specta_type_for_glam_type!(U16Vec4 as [u16; 4]);

    // Implementations for https://docs.rs/glam/latest/glam/i32/index.html
    implement_specta_type_for_glam_type!(IVec2 as [i32; 2]);
    implement_specta_type_for_glam_type!(IVec3 as [i32; 3]);
    implement_specta_type_for_glam_type!(IVec4 as [i32; 4]);

    // Implementations for https://docs.rs/glam/latest/glam/u32/index.html
    implement_specta_type_for_glam_type!(UVec2 as [u32; 2]);
    implement_specta_type_for_glam_type!(UVec3 as [u32; 3]);
    implement_specta_type_for_glam_type!(UVec4 as [u32; 4]);

    // Implementation for https://docs.rs/glam/latest/glam/i64/index.html
    implement_specta_type_for_glam_type!(I64Vec2 as [i64; 2]);
    implement_specta_type_for_glam_type!(I64Vec3 as [i64; 3]);
    implement_specta_type_for_glam_type!(I64Vec4 as [i64; 4]);

    // Implementation for https://docs.rs/glam/latest/glam/u64/index.html
    implement_specta_type_for_glam_type!(U64Vec2 as [u64; 2]);
    implement_specta_type_for_glam_type!(U64Vec3 as [u64; 3]);
    implement_specta_type_for_glam_type!(U64Vec4 as [u64; 4]);

    // implementation for https://docs.rs/glam/latest/glam/usize/index.html
    implement_specta_type_for_glam_type!(USizeVec2 as [usize; 2]);
    implement_specta_type_for_glam_type!(USizeVec3 as [usize; 3]);
    implement_specta_type_for_glam_type!(USizeVec4 as [usize; 4]);

    // Implementation for https://docs.rs/glam/latest/glam/bool/index.html
    implement_specta_type_for_glam_type!(BVec2 as [bool; 2]);
    implement_specta_type_for_glam_type!(BVec3 as [bool; 3]);
    implement_specta_type_for_glam_type!(BVec4 as [bool; 4]);
};

#[cfg(feature = "url")]
impl_as!(url::Url as String);

#[cfg(feature = "either")]
impl<L: Type, R: Type> Type for either::Either<L, R> {
    fn definition(types: &mut TypeCollection) -> DataType {
        DataType::Enum(Enum {
            variants: vec![
                (
                    "Left".into(),
                    EnumVariant::unnamed()
                        .field(Field::new(L::definition(types)))
                        .build(),
                ),
                (
                    "Right".into(),
                    EnumVariant::unnamed()
                        .field(Field::new(R::definition(types)))
                        .build(),
                ),
            ],
            attributes: vec![],
        })
    }
}

#[cfg(feature = "bevy_ecs")]
const _: () = {
    impl Type for bevy_ecs::entity::Entity {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Struct(Struct {
                fields: Fields::Unnamed(UnnamedFields {
                    fields: vec![Field {
                        optional: false,
                        inline: false,
                        deprecated: None,
                        docs: Cow::Borrowed(""),
                        ty: Some(u64::definition(types)),
                        attributes: Vec::new(),
                    }],
                    attributes: Vec::new(),
                }),
                attributes: Vec::new(),
            })
        }
    }
};

#[cfg(feature = "bevy_input")]
const _: () = {
    impl Type for bevy_input::ButtonState {
        fn definition(_: &mut TypeCollection) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    ("Pressed".into(), EnumVariant::unit()),
                    ("Released".into(), EnumVariant::unit()),
                ],
                attributes: vec![],
            })
        }
    }

    impl Type for bevy_input::keyboard::KeyboardInput {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Struct(Struct {
                fields: Fields::Named(NamedFields {
                    fields: vec![
                        (
                            "key_code".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(bevy_input::keyboard::KeyCode::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                        (
                            "logical_key".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(bevy_input::keyboard::Key::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                        (
                            "state".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(bevy_input::ButtonState::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                        (
                            "window".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(bevy_ecs::entity::Entity::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                    ],
                    attributes: Vec::new(),
                }),
                attributes: Vec::new(),
            })
        }
    }

    // Reduced KeyCode and Key to String to avoid redefining a quite large enum (for now)
    impl_as!(
        bevy_input::keyboard::KeyCode as String
        bevy_input::keyboard::Key as String
    );

    impl Type for bevy_input::mouse::MouseButtonInput {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Struct(Struct {
                fields: Fields::Named(NamedFields {
                    fields: vec![
                        (
                            "button".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(bevy_input::mouse::MouseButton::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                        (
                            "state".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(bevy_input::ButtonState::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                        (
                            "window".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(bevy_ecs::entity::Entity::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                    ],
                    attributes: Vec::new(),
                }),
                attributes: Vec::new(),
            })
        }
    }

    impl Type for bevy_input::mouse::MouseButton {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    ("Left".into(), EnumVariant::unit()),
                    ("Right".into(), EnumVariant::unit()),
                    ("Middle".into(), EnumVariant::unit()),
                    ("Back".into(), EnumVariant::unit()),
                    ("Forward".into(), EnumVariant::unit()),
                    (
                        "Other".into(),
                        EnumVariant::unnamed()
                            .field(Field::new(u16::definition(types)))
                            .build(),
                    ),
                ],
                attributes: vec![],
            })
        }
    }

    impl Type for bevy_input::mouse::MouseWheel {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Struct(Struct {
                fields: Fields::Named(NamedFields {
                    fields: vec![
                        (
                            "unit".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(bevy_input::mouse::MouseScrollUnit::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                        (
                            "x".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(f32::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                        (
                            "y".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(f32::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                        (
                            "window".into(),
                            Field {
                                optional: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(bevy_ecs::entity::Entity::definition(types)),
                                attributes: Vec::new(),
                            },
                        ),
                    ],
                    attributes: Vec::new(),
                }),
                attributes: Vec::new(),
            })
        }
    }

    impl Type for bevy_input::mouse::MouseScrollUnit {
        fn definition(_: &mut TypeCollection) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    ("Line".into(), EnumVariant::unit()),
                    ("Pixel".into(), EnumVariant::unit()),
                ],
                attributes: vec![],
            })
        }
    }

    impl Type for bevy_input::mouse::MouseMotion {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Struct(Struct {
                fields: Fields::Named(NamedFields {
                    fields: vec![(
                        "delta".into(),
                        Field {
                            optional: false,

                            inline: false,
                            deprecated: None,
                            docs: Cow::Borrowed(""),
                            ty: Some(glam::Vec2::definition(types)),
                            attributes: Vec::new(),
                        },
                    )],
                    attributes: Vec::new(),
                }),
                attributes: Vec::new(),
            })
        }
    }
};

#[cfg(feature = "camino")]
impl_as!(
    camino::Utf8Path as String
    camino::Utf8PathBuf as String
);

#[cfg(feature = "geojson")]
const _: () = {
    use geojson::{Feature, FeatureCollection, Geometry, Value};

    #[derive(Type)]
    #[specta(rename = "GeoJsonValue", untagged, remote = Value, crate = crate, collect = false)]
    #[allow(dead_code)]
    pub enum GeoJsonValue {
        Point(geojson::PointType),
        MultiPoint(Vec<geojson::PointType>),
        LineString(geojson::LineStringType),
        MultiLineString(Vec<geojson::LineStringType>),
        Polygon(geojson::PolygonType),
        MultiPolygon(Vec<geojson::PolygonType>),
        GeometryCollection(Vec<Geometry>),
    }

    #[derive(Type)]
    #[specta(rename = "GeoJsonGeometry", remote = Geometry, crate = crate, collect = false)]
    #[allow(dead_code)]
    pub struct GeoJsonGeometry {
        pub bbox: Option<geojson::Bbox>,
        pub value: Value,
        pub foreign_members: Option<geojson::JsonObject>,
    }

    #[derive(Type)]
    #[specta(rename = "GeoJsonFeature", remote = Feature, crate = crate, collect = false)]
    #[allow(dead_code)]
    pub struct GeoJsonFeature {
        pub bbox: Option<geojson::Bbox>,
        pub geometry: Option<Geometry>,
        pub id: Option<geojson::feature::Id>,
        pub properties: Option<geojson::JsonObject>,
        pub foreign_members: Option<geojson::JsonObject>,
    }

    #[derive(Type)]
    #[specta(rename = "GeoJsonFeatureCollection", remote = FeatureCollection, crate = crate, collect = false)]
    #[allow(dead_code)]
    pub struct GeoJsonFeatureCollection {
        pub bbox: Option<geojson::Bbox>,
        pub features: Vec<Feature>,
        pub foreign_members: Option<geojson::JsonObject>,
    }

    #[derive(Type)]
    #[specta(rename = "GeoJsonFeatureId", untagged, remote = geojson::feature::Id, crate = crate, collect = false)]
    #[allow(dead_code)]
    pub enum GeoJsonFeatureId {
        String(String),
        Number(serde_json::Number),
    }
};
