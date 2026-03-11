#![allow(unused)]

//! The plan is to try and move these into the ecosystem for the v2 release.
use super::macros::{impl_ndt, impl_ndt_as};
use crate::{
    Type, TypeCollection,
    datatype::{
        self, DataType, Enum, Variant, Field, Fields, NamedFields, Primitive, Reference, Struct,
    },
    r#type::{generics, impls::*},
};

use std::borrow::Cow;

#[cfg(feature = "indexmap")]
impl_ndt_as!(
    indexmap::IndexSet<T> as PrimitiveSet<generics::T>
    indexmap::IndexMap<K, V> as PrimitiveMap<generics::K, generics::V>
);

#[cfg(feature = "bytes")]
impl_ndt_as!(
    bytes::Bytes as [u8]
    bytes::BytesMut as [u8]
);

#[cfg(feature = "serde_json")]
const _: () = {
    use serde_json::{Map, Number, Value};

    impl_ndt_as!(
        serde_json::Map<K, V> as PrimitiveMap<generics::K, generics::V>
    );

    impl_ndt!(
        impl Type for serde_json::Value {
            inline: true;
            build: |types, ndt| {
                ndt.inner = DataType::Enum(Enum {
                    variants: vec![
                        ("Null".into(), Variant::unit()),
                        (
                            "Bool".into(),
                            Variant::unnamed()
                                .field(Field::new(bool::definition(types)))
                                .build(),
                        ),
                        (
                            "Number".into(),
                            Variant::unnamed()
                                .field(Field::new(Number::definition(types)))
                                .build(),
                        ),
                        (
                            "String".into(),
                            Variant::unnamed()
                                .field(Field::new(String::definition(types)))
                                .build(),
                        ),
                        (
                            "Array".into(),
                            Variant::unnamed()
                                .field(Field::new(Vec::<Value>::definition(types)))
                                .build(),
                        ),
                        (
                            "Object".into(),
                            Variant::unnamed()
                                .field(Field::new(Map::<String, Value>::definition(types)))
                                .build(),
                        ),
                    ],
                    attributes: datatype::Attributes::default(),
                })
            }
        }

        impl Type for serde_json::Number {
            inline: true;
            build: |types, ndt| {
                ndt.inner = DataType::Enum(Enum {
                    variants: vec![
                        (
                            "f64".into(),
                            Variant::unnamed()
                                .field(Field::new(DataType::Primitive(Primitive::f64)))
                                .build(),
                        ),
                        (
                            "i64".into(),
                            Variant::unnamed()
                                .field(Field::new(DataType::Primitive(Primitive::i64)))
                                .build(),
                        ),
                        (
                            "u64".into(),
                            Variant::unnamed()
                                .field(Field::new(DataType::Primitive(Primitive::u64)))
                                .build(),
                        ),
                    ],
                    attributes: datatype::Attributes::default(),
                });
            }
        }
    );
};

#[cfg(feature = "serde_yaml")]
const _: () = {
    use serde_yaml::{Number, Value, value::TaggedValue};

    impl_ndt_as!(
        serde_yaml::Mapping as PrimitiveMap<serde_yaml::Value, serde_yaml::Value>
        serde_yaml::value::TaggedValue as PrimitiveMap<String, serde_yaml::Value>
    );

    impl_ndt!(
        impl Type for serde_yaml::Value {
            inline: true;
            build: |types, ndt| {
                ndt.inner = DataType::Enum(Enum {
                    variants: vec![
                        ("Null".into(), Variant::unit()),
                        (
                            "Bool".into(),
                            Variant::unnamed()
                                .field(Field::new(bool::definition(types)))
                                .build(),
                        ),
                        (
                            "Number".into(),
                            Variant::unnamed()
                                .field(Field::new(Number::definition(types)))
                                .build(),
                        ),
                        (
                            "String".into(),
                            Variant::unnamed()
                                .field(Field::new(String::definition(types)))
                                .build(),
                        ),
                        (
                            "Sequence".into(),
                            Variant::unnamed()
                                .field(Field::new(Vec::<Value>::definition(types)))
                                .build(),
                        ),
                        (
                            "Mapping".into(),
                            Variant::unnamed()
                                .field(Field::new(std::collections::BTreeMap::<
                                    serde_yaml::Value,
                                    serde_yaml::Value,
                                >::definition(types)))
                                .build(),
                        ),
                        (
                            "Tagged".into(),
                            Variant::unnamed()
                                .field(Field::new(Box::<TaggedValue>::definition(types)))
                                .build(),
                        ),
                    ],
                    attributes: datatype::Attributes::default(),
                })
            }
        }

        impl Type for serde_yaml::Number {
            inline: true;
            build: |types, ndt| {
                ndt.inner = DataType::Enum(Enum {
                    variants: vec![
                        (
                            "f64".into(),
                            Variant::unnamed()
                                .field(Field::new(DataType::Primitive(Primitive::f64)))
                                .build(),
                        ),
                        (
                            "i64".into(),
                            Variant::unnamed()
                                .field(Field::new(DataType::Primitive(Primitive::i64)))
                                .build(),
                        ),
                        (
                            "u64".into(),
                            Variant::unnamed()
                                .field(Field::new(DataType::Primitive(Primitive::u64)))
                                .build(),
                        ),
                    ],
                    attributes: datatype::Attributes::default(),
                })
            }
        }
    );
};

#[cfg(feature = "toml")]
const _: () = {
    use toml::{Value, value};

    impl_ndt_as!(toml::map::Map<K, V> as PrimitiveMap<generics::K, generics::V>);

    impl_ndt!(
        impl Type for toml::value::Datetime {
            inline: true;
            build: |types, ndt| {
                ndt.inner = DataType::Struct(Struct {
                    fields: Fields::Named(NamedFields {
                        fields: vec![(
                            "v".into(),
                            Field {
                                optional: false,
                                flatten: false,

                                inline: false,
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(String::definition(types)),
                                attributes: datatype::Attributes::default(),
                            },
                        )],
                    }),
                    attributes: datatype::Attributes::default(),
                })
            }
        }

        impl Type for toml::Value {
            inline: true;
            build: |types, ndt| {
                ndt.inner = DataType::Enum(Enum {
                    variants: vec![
                        (
                            "String".into(),
                            Variant::unnamed()
                                .field(Field::new(String::definition(types)))
                                .build(),
                        ),
                        (
                            "Integer".into(),
                            Variant::unnamed()
                                .field(Field::new(i64::definition(types)))
                                .build(),
                        ),
                        (
                            "Float".into(),
                            Variant::unnamed()
                                .field(Field::new(f64::definition(types)))
                                .build(),
                        ),
                        (
                            "Boolean".into(),
                            Variant::unnamed()
                                .field(Field::new(bool::definition(types)))
                                .build(),
                        ),
                        (
                            "Datetime".into(),
                            Variant::unnamed()
                                .field(Field::new(value::Datetime::definition(types)))
                                .build(),
                        ),
                        (
                            "Array".into(),
                            Variant::unnamed()
                                .field(Field::new(Vec::<Value>::definition(types)))
                                .build(),
                        ),
                        (
                            "Table".into(),
                            Variant::unnamed()
                                .field(Field::new(
                                    std::collections::BTreeMap::<String, Value>::definition(types),
                                ))
                                .build(),
                        ),
                    ],
                    attributes: datatype::Attributes::default(),
                })
            }
        }
    );
};

#[cfg(feature = "ulid")]
impl_ndt_as!(ulid::Ulid as str);

#[cfg(feature = "uuid")]
impl_ndt_as!(
    uuid::Uuid as str
    uuid::fmt::Hyphenated as str
);

#[cfg(feature = "chrono")]
#[allow(deprecated)]
const _: () = {
    impl_ndt_as!(
        chrono::NaiveDateTime as str
        chrono::NaiveDate as str
        chrono::NaiveTime as str
        chrono::Duration as str
    );

    // This is special cause of how it ignores the `generics` param to `NamedDataType::init_with_sentinel`
    // These needs generics which also aren't `Type` & aren't in `References` param so `impl_ndt` doesn't work.
    macro_rules! impl_as_str {
        ($module:ident :: $type_name:ident) => {
            fn definition(types: &mut TypeCollection) -> DataType {
                // This API is internal. Use [NamedDataType::register] if you want a custom implementation.
                static SENTINEL: &str = stringify!($module::$type_name);
                static GENERICS: &[(datatype::GenericReference, Cow<'static, str>)] = &[];
                DataType::Reference(datatype::NamedDataType::init_with_sentinel(
                    GENERICS,
                    vec![],
                    true,
                    types,
                    SENTINEL,
                    |types, ndt| {
                        ndt.set_name(::std::borrow::Cow::Borrowed(stringify!($type_name)));
                        ndt.set_module_path(::std::borrow::Cow::Borrowed(stringify!($module)));
                        ndt.inner = str::definition(types);
                    },
                ))
            }
        };
    }

    impl<T: chrono::TimeZone> Type for chrono::Date<T> {
        impl_as_str!(chrono::Date);
    }
    impl<T: chrono::TimeZone> Type for chrono::DateTime<T> {
        impl_as_str!(chrono::DateTime);
    }
};

#[cfg(feature = "time")]
impl_ndt_as!(
    time::PrimitiveDateTime as str
    time::OffsetDateTime as str
    time::Date as str
    time::Time as str
    time::Duration as str
    time::Weekday as str
);

#[cfg(feature = "jiff")]
impl_ndt_as!(
    jiff::Timestamp as str
    jiff::Zoned as str
    jiff::Span as str
    jiff::civil::Date as str
    jiff::civil::Time as str
    jiff::civil::DateTime as str
    jiff::tz::TimeZone as str
);

#[cfg(feature = "bigdecimal")]
impl_ndt_as!(bigdecimal::BigDecimal as str);

// This assumes the `serde-with-str` feature is enabled. Check #26 for more info.
#[cfg(feature = "rust_decimal")]
impl_ndt_as!(rust_decimal::Decimal as str);

#[cfg(feature = "ipnetwork")]
impl_ndt_as!(
    ipnetwork::IpNetwork as str
    ipnetwork::Ipv4Network as str
    ipnetwork::Ipv6Network as str
);

#[cfg(feature = "mac_address")]
impl_ndt_as!(mac_address::MacAddress as str);

#[cfg(feature = "chrono")]
impl_ndt_as!(
    chrono::FixedOffset as str
    chrono::Utc as str
    chrono::Local as str
);

#[cfg(feature = "bson")]
impl_ndt_as!(
    bson::oid::ObjectId as str
    bson::Decimal128 as i128
    bson::DateTime as str
    bson::Uuid as str
);

// TODO: bson::bson
// TODO: bson::Document

#[cfg(feature = "bytesize")]
impl_ndt_as!(bytesize::ByteSize as u64);

#[cfg(feature = "uhlc")]
const _: () = {
    impl_ndt_as!(
        uhlc::NTP64 as u64
        uhlc::ID as std::num::NonZeroU128
    );

    impl_ndt!(
        impl Type for uhlc::Timestamp {
            inline: true;
            build: |types, ndt| {
                ndt.inner = DataType::Struct(Struct {
                    fields: Fields::Named(NamedFields {
                        fields: vec![
                            (
                                "time".into(),
                                Field {
                                    optional: false,
                                    flatten: false,

                                    inline: false,
                                    deprecated: None,
                                    docs: Cow::Borrowed(""),
                                    ty: Some(uhlc::NTP64::definition(types)),
                                    attributes: datatype::Attributes::default(),
                                },
                            ),
                            (
                                "id".into(),
                                Field {
                                    optional: false,
                                    flatten: false,

                                    inline: false,
                                    deprecated: None,
                                    docs: Cow::Borrowed(""),
                                    ty: Some(uhlc::ID::definition(types)),
                                    attributes: datatype::Attributes::default(),
                                },
                            ),
                        ],
                    }),
                    attributes: datatype::Attributes::default(),
                });
            }
        }
    );
};

#[cfg(feature = "glam")]
impl_ndt_as!(
    // Affines
    glam::Affine2 as [f32; 6]
    glam::Affine3A as [f32; 12]

    // Matrices
    glam::Mat2 as [f32; 4]
    glam::Mat3 as [f32; 9]
    glam::Mat3A as [f32; 9]
    glam::Mat4 as [f32; 16]

    // Quaternions
    glam::Quat as [f32; 4]

    // Vectors
    glam::Vec2 as [f32; 2]
    glam::Vec3 as [f32; 3]
    glam::Vec3A as [f32; 3]
    glam::Vec4 as [f32; 4]

    // Affines
    glam::DAffine2 as [f64; 6]
    glam::DAffine3 as [f64; 12]

    // Matrices
    glam::DMat2 as [f64; 4]
    glam::DMat3 as [f64; 9]
    glam::DMat4 as [f64; 16]

    // Quaternions
    glam::DQuat as [f64; 4]

    // Vectors
    glam::DVec2 as [f64; 2]
    glam::DVec3 as [f64; 3]
    glam::DVec4 as [f64; 4]

    // Implementations for https://docs.rs/glam/latest/glam/i8/index.html
    glam::I8Vec2 as [i8; 2]
    glam::I8Vec3 as [i8; 3]
    glam::I8Vec4 as [i8; 4]

    // Implementations for https://docs.rs/glam/latest/glam/u8/index.html
    glam::U8Vec2 as [u8; 2]
    glam::U8Vec3 as [u8; 3]
    glam::U8Vec4 as [u8; 4]

    // Implementations for https://docs.rs/glam/latest/glam/i16/index.html
    glam::I16Vec2 as [i16; 2]
    glam::I16Vec3 as [i16; 3]
    glam::I16Vec4 as [i16; 4]

    // Implementations for https://docs.rs/glam/latest/glam/u16/index.html
    glam::U16Vec2 as [u16; 2]
    glam::U16Vec3 as [u16; 3]
    glam::U16Vec4 as [u16; 4]

    // Implementations for https://docs.rs/glam/latest/glam/i32/index.html
    glam::IVec2 as [i32; 2]
    glam::IVec3 as [i32; 3]
    glam::IVec4 as [i32; 4]

    // Implementations for https://docs.rs/glam/latest/glam/u32/index.html
    glam::UVec2 as [u32; 2]
    glam::UVec3 as [u32; 3]
    glam::UVec4 as [u32; 4]

    // Implementation for https://docs.rs/glam/latest/glam/i64/index.html
    glam::I64Vec2 as [i64; 2]
    glam::I64Vec3 as [i64; 3]
    glam::I64Vec4 as [i64; 4]

    // Implementation for https://docs.rs/glam/latest/glam/u64/index.html
    glam::U64Vec2 as [u64; 2]
    glam::U64Vec3 as [u64; 3]
    glam::U64Vec4 as [u64; 4]

    // implementation for https://docs.rs/glam/latest/glam/usize/index.html
    glam::USizeVec2 as [usize; 2]
    glam::USizeVec3 as [usize; 3]
    glam::USizeVec4 as [usize; 4]

    // Implementation for https://docs.rs/glam/latest/glam/bool/index.html
    glam::BVec2 as [bool; 2]
    glam::BVec3 as [bool; 3]
    glam::BVec4 as [bool; 4]
);

#[cfg(feature = "url")]
impl_ndt_as!(url::Url as str);

#[cfg(feature = "either")]
impl_ndt!(
    impl<L, R> Type for either::Either<L, R> where { L: Type, R: Type } {
        inline: true;
        build: |types, ndt| {
            ndt.inner = DataType::Enum(Enum {
                variants: vec![
                    (
                        "Left".into(),
                        Variant::unnamed()
                            .field(Field::new(
                                datatype::GenericReference::new::<generics::L>().into(),
                            ))
                            .build(),
                    ),
                    (
                        "Right".into(),
                        Variant::unnamed()
                            .field(Field::new(
                                datatype::GenericReference::new::<generics::R>().into(),
                            ))
                            .build(),
                    ),
                ],
                attributes: datatype::Attributes::default(),
            });
        }
    }
);

#[cfg(feature = "bevy_ecs")]
impl_ndt!(
    impl Type for bevy_ecs::entity::Entity {
        inline: true;
        build: |types, ndt| {
            let mut s = Struct::unit();
            s.set_fields(crate::internal::construct::fields_unnamed(
                vec![Field::new(u64::definition(types))],
            ));

            ndt.inner = DataType::Struct(s);
        }
    }
);

#[cfg(feature = "bevy_input")]
const _: () = {
    // Reduced KeyCode and Key to str to avoid redefining a quite large enum (for now)
    impl_ndt_as!(
        bevy_input::keyboard::KeyCode as str
        bevy_input::keyboard::Key as str
    );

    impl_ndt!(
        impl Type for bevy_input::ButtonState {
            inline: true;
            build: |_types, ndt| {
                ndt.inner = DataType::Enum(Enum {
                    variants: vec![
                        ("Pressed".into(), Variant::unit()),
                        ("Released".into(), Variant::unit()),
                    ],
                    attributes: datatype::Attributes::default(),
                });
            }
        }

        impl Type for bevy_input::keyboard::KeyboardInput {
            inline: true;
            build: |types, ndt| {
                let mut s = Struct::unit();
                s.set_fields(crate::internal::construct::fields_named(
                    vec![
                        (
                            "key_code".into(),
                            Field::new(bevy_input::keyboard::KeyCode::definition(types)),
                        ),
                        (
                            "logical_key".into(),
                            Field::new(bevy_input::keyboard::Key::definition(types)),
                        ),
                        (
                            "state".into(),
                            Field::new(bevy_input::ButtonState::definition(types)),
                        ),
                        (
                            "window".into(),
                            Field::new(bevy_ecs::entity::Entity::definition(types)),
                        ),
                    ],
                ));

                ndt.inner = DataType::Struct(s);
            }
        }

        impl Type for bevy_input::mouse::MouseButtonInput {
            inline: true;
            build: |types, ndt| {
                let mut s = Struct::unit();
                s.set_fields(crate::internal::construct::fields_named(
                    vec![
                        (
                            "button".into(),
                            Field::new(bevy_input::mouse::MouseButton::definition(types)),
                        ),
                        (
                            "state".into(),
                            Field::new(bevy_input::ButtonState::definition(types)),
                        ),
                        (
                            "window".into(),
                            Field::new(bevy_ecs::entity::Entity::definition(types)),
                        ),
                    ],
                ));

                ndt.inner = DataType::Struct(s);
            }
        }

        impl Type for bevy_input::mouse::MouseButton {
            inline: true;
            build: |types, ndt| {
                ndt.inner = DataType::Enum(Enum {
                    variants: vec![
                        ("Left".into(), Variant::unit()),
                        ("Right".into(), Variant::unit()),
                        ("Middle".into(), Variant::unit()),
                        ("Back".into(), Variant::unit()),
                        ("Forward".into(), Variant::unit()),
                        (
                            "Other".into(),
                            Variant::unnamed()
                                .field(Field::new(u16::definition(types)))
                                .build(),
                        ),
                    ],
                    attributes: datatype::Attributes::default(),
                });
            }
        }

        impl Type for bevy_input::mouse::MouseWheel {
            inline: true;
            build: |types, ndt| {
                let mut s = Struct::unit();
                s.set_fields(crate::internal::construct::fields_named(
                    vec![
                        (
                            "unit".into(),
                            Field::new(bevy_input::mouse::MouseScrollUnit::definition(types)),
                        ),
                        ("x".into(), Field::new(f32::definition(types))),
                        ("y".into(), Field::new(f32::definition(types))),
                        (
                            "window".into(),
                            Field::new(bevy_ecs::entity::Entity::definition(types)),
                        ),
                    ],
                ));

                ndt.inner = DataType::Struct(s);
            }
        }

        impl Type for bevy_input::mouse::MouseScrollUnit {
            inline: true;
            build: |_types, ndt| {
                ndt.inner = DataType::Enum(Enum {
                    variants: vec![
                        ("Line".into(), Variant::unit()),
                        ("Pixel".into(), Variant::unit()),
                    ],
                    attributes: datatype::Attributes::default(),
                });
            }
        }

        impl Type for bevy_input::mouse::MouseMotion {
            inline: true;
            build: |types, ndt| {
                let mut s = Struct::unit();
                s.set_fields(crate::internal::construct::fields_named(
                    vec![("delta".into(), Field::new(glam::Vec2::definition(types)))],
                ));

                ndt.inner = DataType::Struct(s);
            }
        }
    );
};

#[cfg(feature = "camino")]
impl_ndt_as!(
    camino::Utf8Path as str
    camino::Utf8PathBuf as str
);

#[cfg(feature = "geojson")]
impl_ndt!(
    impl Type for geojson::Value {
        inline: true;
        build: |types, ndt| {
            ndt.inner = DataType::Enum(Enum {
                variants: vec![
                    (
                        "Point".into(),
                        Variant::unnamed()
                            .field(Field::new(geojson::PointType::definition(types)))
                            .build(),
                    ),
                    (
                        "MultiPoint".into(),
                        Variant::unnamed()
                            .field(Field::new(Vec::<geojson::PointType>::definition(types)))
                            .build(),
                    ),
                    (
                        "LineString".into(),
                        Variant::unnamed()
                            .field(Field::new(geojson::LineStringType::definition(types)))
                            .build(),
                    ),
                    (
                        "MultiLineString".into(),
                        Variant::unnamed()
                            .field(Field::new(Vec::<geojson::LineStringType>::definition(
                                types,
                            )))
                            .build(),
                    ),
                    (
                        "Polygon".into(),
                        Variant::unnamed()
                            .field(Field::new(geojson::PolygonType::definition(types)))
                            .build(),
                    ),
                    (
                        "MultiPolygon".into(),
                        Variant::unnamed()
                            .field(Field::new(Vec::<geojson::PolygonType>::definition(types)))
                            .build(),
                    ),
                    (
                        "GeometryCollection".into(),
                        Variant::unnamed()
                            .field(Field::new(Vec::<geojson::Geometry>::definition(types)))
                            .build(),
                    ),
                ],
                attributes: datatype::Attributes::default(),
            });
        }
    }

    impl Type for geojson::Geometry {
        inline: true;
        build: |types, ndt| {
            let mut s = Struct::unit();
            s.set_fields(crate::internal::construct::fields_named(
                vec![
                    (
                        "bbox".into(),
                        Field::new(Option::<geojson::Bbox>::definition(types)),
                    ),
                    ("value".into(), Field::new(geojson::Value::definition(types))),
                    (
                        "foreign_members".into(),
                        Field::new(Option::<geojson::JsonObject>::definition(types)),
                    ),
                ],
            ));

            ndt.inner = DataType::Struct(s);
        }
    }

    impl Type for geojson::Feature {
        inline: true;
        build: |types, ndt| {
            let mut s = Struct::unit();
            s.set_fields(crate::internal::construct::fields_named(
                vec![
                    (
                        "bbox".into(),
                        Field::new(Option::<geojson::Bbox>::definition(types)),
                    ),
                    (
                        "geometry".into(),
                        Field::new(Option::<geojson::Geometry>::definition(types)),
                    ),
                    (
                        "id".into(),
                        Field::new(Option::<geojson::feature::Id>::definition(types)),
                    ),
                    (
                        "properties".into(),
                        Field::new(Option::<geojson::JsonObject>::definition(types)),
                    ),
                    (
                        "foreign_members".into(),
                        Field::new(Option::<geojson::JsonObject>::definition(types)),
                    ),
                ],
            ));

            ndt.inner = DataType::Struct(s);
        }
    }

    impl Type for geojson::FeatureCollection {
        inline: true;
        build: |types, ndt| {
            let mut s = Struct::unit();
            s.set_fields(crate::internal::construct::fields_named(
                vec![
                    (
                        "bbox".into(),
                        Field::new(Option::<geojson::Bbox>::definition(types)),
                    ),
                    (
                        "features".into(),
                        Field::new(Vec::<geojson::Feature>::definition(types)),
                    ),
                    (
                        "foreign_members".into(),
                        Field::new(Option::<geojson::JsonObject>::definition(types)),
                    ),
                ],
            ));

            ndt.inner = DataType::Struct(s);
        }
    }

    impl Type for geojson::feature::Id {
        inline: true;
        build: |types, ndt| {
            ndt.inner = DataType::Enum(Enum {
                variants: vec![
                    (
                        "String".into(),
                        Variant::unnamed()
                            .field(Field::new(str::definition(types)))
                            .build(),
                    ),
                    (
                        "Number".into(),
                        Variant::unnamed()
                            .field(Field::new(serde_json::Number::definition(types)))
                            .build(),
                    ),
                ],
                attributes: datatype::Attributes::default(),
            });
        }
    }
);

#[cfg(feature = "geozero")]
impl_ndt!(
    impl Type for geozero::mvt::Tile {
        inline: true;
        build: |types, ndt| {
            let mut s = Struct::unit();
            s.set_fields(crate::internal::construct::fields_named(
                vec![(
                    "layers".into(),
                    Field::new(Vec::<geozero::mvt::tile::Layer>::definition(types)),
                )],
            ));

            ndt.inner = DataType::Struct(s);
        }
    }

    impl Type for geozero::mvt::tile::Value {
        inline: true;
        build: |types, ndt| {
            let mut s = Struct::unit();
            s.set_fields(crate::internal::construct::fields_named(
                vec![
                    (
                        "string_value".into(),
                        Field::new(Option::<String>::definition(types)),
                    ),
                    (
                        "float_value".into(),
                        Field::new(Option::<f32>::definition(types)),
                    ),
                    (
                        "double_value".into(),
                        Field::new(Option::<f64>::definition(types)),
                    ),
                    ("int_value".into(), Field::new(Option::<i64>::definition(types))),
                    (
                        "uint_value".into(),
                        Field::new(Option::<u64>::definition(types)),
                    ),
                    (
                        "sint_value".into(),
                        Field::new(Option::<i64>::definition(types)),
                    ),
                    (
                        "bool_value".into(),
                        Field::new(Option::<bool>::definition(types)),
                    ),
                ],
            ));

            ndt.inner = DataType::Struct(s);
        }
    }

    impl Type for geozero::mvt::tile::Feature {
        inline: true;
        build: |types, ndt| {
            let mut s = Struct::unit();
            s.set_fields(crate::internal::construct::fields_named(
                vec![
                    ("id".into(), Field::new(Option::<u64>::definition(types))),
                    ("tags".into(), Field::new(Vec::<u32>::definition(types))),
                    ("type".into(), Field::new(Option::<i32>::definition(types))),
                    ("geometry".into(), Field::new(Vec::<u32>::definition(types))),
                ],
            ));

            ndt.inner = DataType::Struct(s);
        }
    }

    impl Type for geozero::mvt::tile::Layer {
        inline: true;
        build: |types, ndt| {
            let mut s = Struct::unit();
            s.set_fields(crate::internal::construct::fields_named(
                vec![
                    ("version".into(), Field::new(u32::definition(types))),
                    ("name".into(), Field::new(String::definition(types))),
                    (
                        "features".into(),
                        Field::new(Vec::<geozero::mvt::tile::Feature>::definition(types)),
                    ),
                    ("keys".into(), Field::new(Vec::<String>::definition(types))),
                    (
                        "values".into(),
                        Field::new(Vec::<geozero::mvt::tile::Value>::definition(types)),
                    ),
                    ("extent".into(), Field::new(Option::<u32>::definition(types))),
                ],
            ));

            ndt.inner = DataType::Struct(s);
        }
    }

    impl Type for geozero::mvt::tile::GeomType {
        inline: true;
        build: |_types, ndt| {
            ndt.inner = DataType::Enum(Enum {
                variants: vec![
                    ("Unknown".into(), Variant::unit()),
                    ("Point".into(), Variant::unit()),
                    ("Linestring".into(), Variant::unit()),
                    ("Polygon".into(), Variant::unit()),
                ],
                attributes: datatype::Attributes::default(),
            });
        }
    }
);
