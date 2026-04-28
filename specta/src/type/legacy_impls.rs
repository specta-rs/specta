use crate::{
    Type, Types,
    datatype::{
        self, DataType, Enum, Field, Fields, List, NamedFields, Primitive, Reference, Struct,
        Variant,
    },
    r#type::{impls::*, macros::impl_ndt},
};

// `String` requires `std` feature, while `str` does not.
// We can't use `str` in a lot of places because it's not `Sized`, so this bridges that.
pub struct String;
impl Type for String {
    fn definition(types: &mut Types) -> DataType {
        str::definition(types)
    }
}

#[cfg(feature = "indexmap")]
#[cfg_attr(docsrs, doc(cfg(feature = "indexmap")))]
impl_ndt!(
    indexmap::IndexSet<T> as PrimitiveSet<T> = inline_passthrough;
    indexmap::IndexMap<K, V> as PrimitiveMap<K, V> = inline_passthrough;
);

#[cfg(feature = "ordered-float")]
#[cfg_attr(docsrs, doc(cfg(feature = "ordered-float")))]
impl_ndt!(
    ordered_float::OrderedFloat<T> where { T: Type + ordered_float::FloatCore } as T = inline;
    ordered_float::NotNan<T> where { T: Type + ordered_float::FloatCore } as T = inline;
);

#[cfg(feature = "heapless")]
#[cfg_attr(docsrs, doc(cfg(feature = "heapless")))]
impl_ndt!(
    // Sequential containers
    heapless::Vec<T> <T, const N: usize, LenT> where { T: Type, LenT: heapless::LenType } as [T; N];
    heapless::Deque<T> <T, const N: usize> where { T: Type } as [T; N];
    heapless::HistoryBuf<T> <T, const N: usize> where { T: Type } as [T; N];
    heapless::BinaryHeap<T, K> <T, K, const N: usize> where { T: Type + Ord, K: heapless::binary_heap::Kind } as [T; N];

    // Sets
    heapless::IndexSet<T, S> <T, S, const N: usize> where { T: Type + Eq + core::hash::Hash, S: core::hash::BuildHasher } as PrimitiveSet<T> = inline_passthrough;

    // Maps
    heapless::IndexMap<K, V, S> <K, V, S, const N: usize> where { K: Type + Eq + core::hash::Hash, V: Type, S: core::hash::BuildHasher } as PrimitiveMap<K, V> = inline_passthrough;
    heapless::LinearMap<K, V> <K, V, const N: usize> where { K: Type + Eq, V: Type } as PrimitiveMap<K, V> = inline_passthrough;

    // String container
    heapless::String <const N: usize, LenT> where { LenT: heapless::LenType } as str = inline;
);

#[cfg(feature = "semver")]
#[cfg_attr(docsrs, doc(cfg(feature = "semver")))]
impl_ndt!(
     semver::Version as str = inline;
     semver::VersionReq  as str = inline;
     semver::Comparator  as str = inline;
);

#[cfg(feature = "smol_str")]
#[cfg_attr(docsrs, doc(cfg(feature = "smol_str")))]
impl_ndt!(smol_str::SmolStr as str = inline);

#[cfg(feature = "arrayvec")]
#[cfg_attr(docsrs, doc(cfg(feature = "arrayvec")))]
impl_ndt!(
    arrayvec::ArrayString <const N: usize> as str = inline;
    arrayvec::ArrayVec<T> <T, const N: usize> as [T; N] = inline_passthrough;
);

#[cfg(feature = "smallvec")]
#[cfg_attr(docsrs, doc(cfg(feature = "smallvec")))]
impl_ndt!(smallvec::SmallVec<T> where { T: smallvec::Array + Type } as T);

#[cfg(feature = "bytes")]
#[cfg_attr(docsrs, doc(cfg(feature = "bytes")))]
impl_ndt!(
    bytes::Bytes as [u8] = inline;
    bytes::BytesMut as [u8] = inline;
);

#[cfg(feature = "serde_json")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde_json")))]
const _: () = {
    use serde_json::{Map, Number, Value};

    impl_ndt!(
        serde_json::Map<K, V> as PrimitiveMap<K, V> = inline_passthrough;
        serde_json::Value as SerdeValue = inline;
        serde_json::Number as SerdeNumber = inline;
    );

    struct SerdeValue;
    impl Type for SerdeValue {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
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
                            .field(Field::new(str::definition(types)))
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
                attributes: Default::default(),
            })
        }
    }

    struct SerdeNumber;
    impl Type for SerdeNumber {
        fn definition(_: &mut Types) -> DataType {
            DataType::Enum(Enum {
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
};

#[cfg(feature = "serde_yaml")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde_yaml")))]
const _: () = {
    use serde_yaml::{Mapping, Number, Value, value::TaggedValue};

    impl_ndt!(
        serde_yaml::Mapping as PrimitiveMap<Value, Value> = inline_passthrough;
        serde_yaml::Value as SerdeYamlValue = inline;
        serde_yaml::Number as SerdeYamlNumber = inline;
        serde_yaml::value::TaggedValue as SerdeYamlTaggedValue = inline;
    );

    struct SerdeYamlValue;
    impl Type for SerdeYamlValue {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
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
                            .field(Field::new(str::definition(types)))
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
                            .field(Field::new(Mapping::definition(types)))
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

    struct SerdeYamlNumber;
    impl Type for SerdeYamlNumber {
        fn definition(_: &mut Types) -> DataType {
            DataType::Enum(Enum {
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

    struct SerdeYamlTaggedValue;
    impl Type for SerdeYamlTaggedValue {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field("tag", Field::new(str::definition(types)))
                .field("value", Field::new(Value::definition(types)))
                .build()
        }
    }
};
#[cfg(feature = "toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
const _: () = {
    use toml::{Value, value};

    impl_ndt!(
        toml::map::Map<K, V> as PrimitiveMap<K, V> = inline_passthrough;
        toml::value::Datetime as TomlDatetime = inline;
        toml::Value as TomlValue = inline;
    );

    struct TomlDatetime;
    impl Type for TomlDatetime {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field("v", Field::new(str::definition(types)))
                .build()
        }
    }

    struct TomlValue;
    impl Type for TomlValue {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    (
                        "String".into(),
                        Variant::unnamed()
                            .field(Field::new(str::definition(types)))
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
                            .field(Field::new(toml::map::Map::<String, Value>::definition(
                                types,
                            )))
                            .build(),
                    ),
                ],
                attributes: datatype::Attributes::default(),
            })
        }
    }
};

#[cfg(feature = "ulid")]
#[cfg_attr(docsrs, doc(cfg(feature = "ulid")))]
impl_ndt!(ulid::Ulid as str = inline);

#[cfg(feature = "uuid")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
impl_ndt!(
    uuid::Uuid as str = inline;
    uuid::fmt::Braced as str = inline;
    uuid::fmt::Hyphenated as str = inline;
    uuid::fmt::Simple as str = inline;
    uuid::fmt::Urn as str = inline;
);

#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[allow(deprecated)]
const _: () = {
    impl_ndt!(
        chrono::NaiveDateTime as str = inline;
        chrono::NaiveDate as str = inline;
        chrono::NaiveTime as str = inline;
        chrono::Duration as str = inline;
        chrono::FixedOffset as str = inline;
        chrono::Utc as str = inline;
        chrono::Local as str = inline;
        chrono::Weekday as str = inline;
        chrono::Month as str = inline;
        chrono::Date<T> where { T: Type + chrono::TimeZone } as str = inline;
        chrono::DateTime<T> where { T: Type + chrono::TimeZone } as str = inline;
    );
};

#[cfg(feature = "time")]
#[cfg_attr(docsrs, doc(cfg(feature = "time")))]
impl_ndt!(
    time::PrimitiveDateTime as str;
    time::OffsetDateTime as str;
    time::Date as str;
    time::Time as str;
    time::Duration as str;
    time::Weekday as str;
);

#[cfg(feature = "jiff")]
#[cfg_attr(docsrs, doc(cfg(feature = "jiff")))]
impl_ndt!(
    jiff::Timestamp as str;
    jiff::Zoned as str;
    jiff::SignedDuration as str;
    jiff::civil::Date as str;
    jiff::civil::Time as str;
    jiff::civil::DateTime as str;
    jiff::civil::ISOWeekDate as str;
    jiff::tz::TimeZone as str;
);

#[cfg(feature = "bigdecimal")]
#[cfg_attr(docsrs, doc(cfg(feature = "bigdecimal")))]
impl_ndt!(bigdecimal::BigDecimal as str = inline);

// This assumes the `serde-with-str` feature is enabled. Check #26 for more info.
#[cfg(feature = "rust_decimal")]
#[cfg_attr(docsrs, doc(cfg(feature = "rust_decimal")))]
impl_ndt!(rust_decimal::Decimal as str = inline);

#[cfg(feature = "ipnetwork")]
#[cfg_attr(docsrs, doc(cfg(feature = "ipnetwork")))]
impl_ndt!(
    ipnetwork::IpNetwork as str = inline;
    ipnetwork::Ipv4Network as str = inline;
    ipnetwork::Ipv6Network as str = inline;
);

#[cfg(feature = "mac_address")]
#[cfg_attr(docsrs, doc(cfg(feature = "mac_address")))]
impl_ndt!(mac_address::MacAddress as str = inline);

#[cfg(feature = "bson")]
#[cfg_attr(docsrs, doc(cfg(feature = "bson")))]
const _: () = {
    impl_ndt!(
        bson::oid::ObjectId as BsonObjectId = inline;
        bson::Decimal128 as BsonDecimal128 = inline;
        bson::DateTime as BsonDateTime = inline;
        bson::Uuid as str = inline;
        bson::Timestamp as BsonTimestamp = inline;
        bson::Binary as BsonBinary = inline;
        bson::Regex as BsonRegex = inline;
        bson::JavaScriptCodeWithScope as BsonJavaScriptCodeWithScope = inline;
        bson::DbPointer as BsonDbPointer = inline;
        bson::Document as PrimitiveMap<String, bson::Bson> = inline;
        bson::Bson as Bson = inline;
    );

    struct BsonObjectId;
    impl Type for BsonObjectId {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field("$oid", Field::new(str::definition(types)))
                .build()
        }
    }

    struct BsonDecimal128;
    impl Type for BsonDecimal128 {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field("$numberDecimal", Field::new(str::definition(types)))
                .build()
        }
    }

    struct BsonDateTime;
    impl Type for BsonDateTime {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field("$date", Field::new(str::definition(types)))
                .build()
        }
    }

    struct BsonTimestamp;
    impl Type for BsonTimestamp {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "$timestamp",
                    Field::new(
                        Struct::named()
                            .field("t", Field::new(u32::definition(types)))
                            .field("i", Field::new(u32::definition(types)))
                            .build(),
                    ),
                )
                .build()
        }
    }

    struct BsonBinary;
    impl Type for BsonBinary {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "$binary",
                    Field::new(
                        Struct::named()
                            .field("base64", Field::new(str::definition(types)))
                            .field("subType", Field::new(str::definition(types)))
                            .build(),
                    ),
                )
                .build()
        }
    }

    struct BsonRegex;
    impl Type for BsonRegex {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field("$regex", Field::new(str::definition(types)))
                .field("$options", Field::new(str::definition(types)))
                .build()
        }
    }

    struct BsonJavaScriptCode;
    impl Type for BsonJavaScriptCode {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field("$code", Field::new(str::definition(types)))
                .build()
        }
    }

    struct BsonJavaScriptCodeWithScope;
    impl Type for BsonJavaScriptCodeWithScope {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field("$code", Field::new(str::definition(types)))
                .field("$scope", Field::new(bson::Document::definition(types)))
                .build()
        }
    }

    struct BsonDbPointer;
    impl Type for BsonDbPointer {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "$dbPointer",
                    Field::new(
                        Struct::named()
                            .field("$ref", Field::new(str::definition(types)))
                            .field("$id", Field::new(bson::oid::ObjectId::definition(types)))
                            .build(),
                    ),
                )
                .build()
        }
    }

    struct Bson;
    impl Type for Bson {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    (
                        "Double".into(),
                        Variant::unnamed()
                            .field(Field::new(f64::definition(types)))
                            .build(),
                    ),
                    (
                        "String".into(),
                        Variant::unnamed()
                            .field(Field::new(str::definition(types)))
                            .build(),
                    ),
                    (
                        "Array".into(),
                        Variant::unnamed()
                            .field(Field::new(Vec::<bson::Bson>::definition(types)))
                            .build(),
                    ),
                    (
                        "Document".into(),
                        Variant::unnamed()
                            .field(Field::new(bson::Document::definition(types)))
                            .build(),
                    ),
                    (
                        "Boolean".into(),
                        Variant::unnamed()
                            .field(Field::new(bool::definition(types)))
                            .build(),
                    ),
                    ("Null".into(), Variant::unit()),
                    (
                        "RegularExpression".into(),
                        Variant::unnamed()
                            .field(Field::new(bson::Regex::definition(types)))
                            .build(),
                    ),
                    (
                        "JavaScriptCode".into(),
                        Variant::unnamed()
                            .field(Field::new(BsonJavaScriptCode::definition(types)))
                            .build(),
                    ),
                    (
                        "JavaScriptCodeWithScope".into(),
                        Variant::unnamed()
                            .field(Field::new(bson::JavaScriptCodeWithScope::definition(types)))
                            .build(),
                    ),
                    (
                        "Int32".into(),
                        Variant::unnamed()
                            .field(Field::new(i32::definition(types)))
                            .build(),
                    ),
                    (
                        "Int64".into(),
                        Variant::unnamed()
                            .field(Field::new(f64::definition(types)))
                            .build(),
                    ),
                    (
                        "Timestamp".into(),
                        Variant::unnamed()
                            .field(Field::new(bson::Timestamp::definition(types)))
                            .build(),
                    ),
                    (
                        "Binary".into(),
                        Variant::unnamed()
                            .field(Field::new(bson::Binary::definition(types)))
                            .build(),
                    ),
                    (
                        "ObjectId".into(),
                        Variant::unnamed()
                            .field(Field::new(bson::oid::ObjectId::definition(types)))
                            .build(),
                    ),
                    (
                        "DateTime".into(),
                        Variant::unnamed()
                            .field(Field::new(bson::DateTime::definition(types)))
                            .build(),
                    ),
                    (
                        "Symbol".into(),
                        Variant::unnamed()
                            .field(Field::new(
                                Struct::named()
                                    .field("$symbol", Field::new(str::definition(types)))
                                    .build(),
                            ))
                            .build(),
                    ),
                    (
                        "Decimal128".into(),
                        Variant::unnamed()
                            .field(Field::new(bson::Decimal128::definition(types)))
                            .build(),
                    ),
                    (
                        "Undefined".into(),
                        Variant::unnamed()
                            .field(Field::new(
                                Struct::named()
                                    .field("$undefined", Field::new(bool::definition(types)))
                                    .build(),
                            ))
                            .build(),
                    ),
                    (
                        "MaxKey".into(),
                        Variant::unnamed()
                            .field(Field::new(
                                Struct::named()
                                    .field("$maxKey", Field::new(u8::definition(types)))
                                    .build(),
                            ))
                            .build(),
                    ),
                    (
                        "MinKey".into(),
                        Variant::unnamed()
                            .field(Field::new(
                                Struct::named()
                                    .field("$minKey", Field::new(u8::definition(types)))
                                    .build(),
                            ))
                            .build(),
                    ),
                    (
                        "DbPointer".into(),
                        Variant::unnamed()
                            .field(Field::new(bson::DbPointer::definition(types)))
                            .build(),
                    ),
                ],
                attributes: datatype::Attributes::default(),
            })
        }
    }
};

// Technically this can be u64 for formats not marked as human readable in Serde.
// but we have no way of inspecting that as it's runtime. This is the most common output.
#[cfg(feature = "bytesize")]
#[cfg_attr(docsrs, doc(cfg(feature = "bytesize")))]
impl_ndt!(bytesize::ByteSize as String);

#[cfg(feature = "uhlc")]
#[cfg_attr(docsrs, doc(cfg(feature = "uhlc")))]
const _: () = {
    impl_ndt!(
        uhlc::NTP64 as u64 = inline;
        uhlc::ID as std::num::NonZeroU128 = inline;
        uhlc::Timestamp as UhlcTimestamp = inline;
    );

    struct UhlcTimestamp;
    impl Type for UhlcTimestamp {
        fn definition(types: &mut Types) -> DataType {
            DataType::Struct(Struct {
                fields: Fields::Named(NamedFields {
                    fields: vec![
                        (
                            "time".into(),
                            Field {
                                optional: false,
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
                                deprecated: None,
                                docs: Cow::Borrowed(""),
                                ty: Some(uhlc::ID::definition(types)),
                                attributes: datatype::Attributes::default(),
                            },
                        ),
                    ],
                }),
                attributes: datatype::Attributes::default(),
            })
        }
    }
};

#[cfg(feature = "glam")]
#[cfg_attr(docsrs, doc(cfg(feature = "glam")))]
impl_ndt!(
    // Affines
    glam::Affine2 as [f32; 6] = inline;
    glam::Affine3A as [f32; 12] = inline;

    glam::DAffine2 as [f64; 6] = inline;
    glam::DAffine3 as [f64; 12] = inline;

    // Matrices
    glam::Mat2 as [f32; 4] = inline;
    glam::Mat3 as [f32; 9] = inline;
    glam::Mat3A as [f32; 9] = inline;
    glam::Mat4 as [f32; 16] = inline;

    glam::DMat2 as [f64; 4] = inline;
    glam::DMat3 as [f64; 9] = inline;
    glam::DMat4 as [f64; 16] = inline;

    // Quaternions
    glam::Quat as [f32; 4] = inline;

    glam::DQuat as [f64; 4] = inline;

    // Vectors
    glam::Vec2 as [f32; 2] = inline;
    glam::Vec3 as [f32; 3] = inline;
    glam::Vec3A as [f32; 3] = inline;
    glam::Vec4 as [f32; 4] = inline;

    glam::DVec2 as [f64; 2] = inline;
    glam::DVec3 as [f64; 3] = inline;
    glam::DVec4 as [f64; 4] = inline;

    // Implementation for https://docs.rs/glam/latest/glam/bool/index.html
    glam::BVec2 as [bool; 2] = inline;
    glam::BVec3 as [bool; 3] = inline;
    glam::BVec3A as [bool; 3] = inline;
    glam::BVec4 as [bool; 4] = inline;
    glam::BVec4A as [bool; 4] = inline;

    // Implementations for https://docs.rs/glam/latest/glam/i8/index.html
    glam::I8Vec2 as [i8; 2] = inline;
    glam::I8Vec3 as [i8; 3] = inline;
    glam::I8Vec4 as [i8; 4] = inline;

    // Implementations for https://docs.rs/glam/latest/glam/u8/index.html
    glam::U8Vec2 as [u8; 2] = inline;
    glam::U8Vec3 as [u8; 3] = inline;
    glam::U8Vec4 as [u8; 4] = inline;

    // Implementations for https://docs.rs/glam/latest/glam/i16/index.html
    glam::I16Vec2 as [i16; 2] = inline;
    glam::I16Vec3 as [i16; 3] = inline;
    glam::I16Vec4 as [i16; 4] = inline;

    // Implementations for https://docs.rs/glam/latest/glam/u16/index.html
    glam::U16Vec2 as [u16; 2] = inline;
    glam::U16Vec3 as [u16; 3] = inline;
    glam::U16Vec4 as [u16; 4] = inline;

    // Implementations for https://docs.rs/glam/latest/glam/u32/index.html
    glam::UVec2 as [u32; 2] = inline;
    glam::UVec3 as [u32; 3] = inline;
    glam::UVec4 as [u32; 4] = inline;

    // Implementations for https://docs.rs/glam/latest/glam/i32/index.html
    glam::IVec2 as [i32; 2] = inline;
    glam::IVec3 as [i32; 3] = inline;
    glam::IVec4 as [i32; 4] = inline;

    // Implementation for https://docs.rs/glam/latest/glam/i64/index.html
    glam::I64Vec2 as [i64; 2] = inline;
    glam::I64Vec3 as [i64; 3] = inline;
    glam::I64Vec4 as [i64; 4] = inline;

    // Implementation for https://docs.rs/glam/latest/glam/u64/index.html
    glam::U64Vec2 as [u64; 2] = inline;
    glam::U64Vec3 as [u64; 3] = inline;
    glam::U64Vec4 as [u64; 4] = inline;

    // implementation for https://docs.rs/glam/latest/glam/usize/index.html
    glam::USizeVec2 as [usize; 2] = inline;
    glam::USizeVec3 as [usize; 3] = inline;
    glam::USizeVec4 as [usize; 4] = inline;

    // implementation for https://docs.rs/glam/latest/glam/isize/index.html
    glam::ISizeVec2 as [isize; 2] = inline;
    glam::ISizeVec3 as [isize; 3] = inline;
    glam::ISizeVec4 as [isize; 4] = inline;
);

#[cfg(feature = "url")]
#[cfg_attr(docsrs, doc(cfg(feature = "url")))]
const _: () = {
    impl_ndt!(
        url::Url as str = inline;
        url::Host as UrlHost = inline;
    );

    struct UrlHost;
    impl Type for UrlHost {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    (
                        "Domain".into(),
                        Variant::unnamed()
                            .field(Field::new(String::definition(types)))
                            .build(),
                    ),
                    (
                        "Ipv4".into(),
                        Variant::unnamed()
                            .field(Field::new(std::net::Ipv4Addr::definition(types)))
                            .build(),
                    ),
                    (
                        "Ipv6".into(),
                        Variant::unnamed()
                            .field(Field::new(std::net::Ipv6Addr::definition(types)))
                            .build(),
                    ),
                ],
                attributes: datatype::Attributes::default(),
            })
        }
    }
};

#[cfg(feature = "either")]
#[cfg_attr(docsrs, doc(cfg(feature = "either")))]
const _: () = {
    impl_ndt!(either::Either<L, R> as Either<L, R> = inline);

    struct Either<L, R>(std::marker::PhantomData<(L, R)>);
    impl<L: Type, R: Type> Type for Either<L, R> {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    (
                        "Left".into(),
                        Variant::unnamed()
                            .field(Field::new(L::definition(types)))
                            .build(),
                    ),
                    (
                        "Right".into(),
                        Variant::unnamed()
                            .field(Field::new(R::definition(types)))
                            .build(),
                    ),
                ],
                attributes: datatype::Attributes::default(),
            })
        }
    }
};

#[cfg(feature = "error-stack")]
#[cfg_attr(docsrs, doc(cfg(feature = "error-stack")))]
const _: () = {
    impl_ndt!(
        error_stack::Report<C> where { C: std::error::Error + Send + Sync + 'static } as ErrorStackReport;
    );

    //     impl<C: std::error::Error + Send + Sync + 'static> Type for error_stack::Report<C> {
    //         fn definition(types: &mut Types) -> DataType {
    //             report_definition(types)
    //         }
    //     }
    impl<C: std::error::Error + Send + Sync + 'static> Type for error_stack::Report<[C]> {
        fn definition(types: &mut Types) -> DataType {
            error_stack::Report::<C>::definition(types)
        }
    }

    // impl<

    struct ErrorStackReport;
    impl Type for ErrorStackReport {
        fn definition(types: &mut Types) -> DataType {
            // DataType::List(List::new(ErrorStackContext::definition(types)))
            todo!();
        }
    }

    // struct ErrorStackContext;
    // impl Type for ErrorStackContext {
    //     fn definition(types: &mut Types) -> DataType {
    //         Struct::named()
    //             .field("context", Field::new(str::definition(types)))
    //             .field(
    //                 "attachments",
    //                 Field::new(DataType::List(List::new(str::definition(types)))),
    //             )
    //             .field(
    //                 "sources",
    //                 Field::new(DataType::List(List::new(ErrorStackContext::definition(
    //                     types,
    //                 )))),
    //             )
    //             .build()
    //     }
    // }

    //     struct ErrorStackContext;
    //     impl Type for ErrorStackContext {
    //         fn definition(types: &mut Types) -> DataType {
    //             static SENTINEL: &str = "error_stack::ErrorStackContext";
    //             static GENERICS: &[datatype::GenericDefinition] = &[];
    //             DataType::Reference(datatype::NamedDataType::init_with_sentinel(
    //                 SENTINEL,
    //                 GENERICS,
    //                 &[],
    //                 false,
    //                 false,
    //                 false,
    //                 types,
    //                 |types, ndt| {
    //                     ndt.name = ::std::borrow::Cow::Borrowed("ErrorStackContext");
    //                     ndt.module_path = ::std::borrow::Cow::Borrowed("error_stack");
    //                     let attachments = DataType::List(List::new(str::definition(types)));
    //                     let sources = DataType::List(List::new(ErrorStackContext::definition(types)));
    //                     ndt.ty = Some(
    //                         Struct::named()
    //                             .field("context", Field::new(str::definition(types)))
    //                             .field("attachments", Field::new(attachments))
    //                             .field("sources", Field::new(sources))
    //                             .build(),
    //                     );
    //                 },
    //                 |types| {
    //                     Struct::named()
    //                         .field("context", Field::new(str::definition(types)))
    //                         .field(
    //                             "attachments",
    //                             Field::new(DataType::List(List::new(str::definition(types)))),
    //                         )
    //                         .field(
    //                             "sources",
    //                             Field::new(DataType::List(List::new(ErrorStackContext::definition(
    //                                 types,
    //                             )))),
    //                         )
    //                         .build()
    //                 },
    //             ))
    //         }
    //     }
    //     fn report_definition(types: &mut Types) -> DataType {
    //         static SENTINEL: &str = "error_stack::Report";
    //         static GENERICS: &[datatype::GenericDefinition] = &[];
    //         DataType::Reference(datatype::NamedDataType::init_with_sentinel(
    //             SENTINEL,
    //             GENERICS,
    //             &[],
    //             false,
    //             false,
    //             false,
    //             types,
    //             |types, ndt| {
    //                 ndt.name = ::std::borrow::Cow::Borrowed("Report");
    //                 ndt.module_path = ::std::borrow::Cow::Borrowed("error_stack");
    //                 ndt.ty = Some(DataType::List(List::new(ErrorStackContext::definition(
    //                     types,
    //                 ))));
    //             },
    //             |types| DataType::List(List::new(ErrorStackContext::definition(types))),
    //         ))
    //     }
};

// #[cfg(feature = "bevy_ecs")]
// #[cfg_attr(docsrs, doc(cfg(feature = "bevy_ecs")))]
// impl_ndt!(
//     impl Type for bevy_ecs::entity::Entity {
//         inline: true;
//         build: |types, ndt| {
//             ndt.ty = Struct::unnamed().field(Field::new(u64::definition(types))).build();
//         }
//     }
// );

// #[cfg(feature = "bevy_input")]
// #[cfg_attr(docsrs, doc(cfg(feature = "bevy_input")))]
// const _: () = {
//     // Reduced KeyCode and Key to str to avoid redefining a quite large enum (for now)
//     impl_ndt!(
//         bevy_input::keyboard::KeyCode as str
//         bevy_input::keyboard::Key as str
//     );

//     impl_ndt!(
//         impl Type for bevy_input::ButtonState {
//             inline: true;
//             build: |_types, ndt| {
//                 ndt.ty = DataType::Enum(Enum {
//                     variants: vec![
//                         ("Pressed".into(), Variant::unit()),
//                         ("Released".into(), Variant::unit()),
//                     ],
//                     attributes: datatype::Attributes::default(),
//                 });
//             }
//         }

//         impl Type for bevy_input::keyboard::KeyboardInput {
//             inline: true;
//             build: |types, ndt| {
//                 ndt.ty = Struct::named()
//                     .field(
//                         "key_code",
//                         Field::new(bevy_input::keyboard::KeyCode::definition(types)),
//                     )
//                     .field(
//                         "logical_key",
//                         Field::new(bevy_input::keyboard::Key::definition(types)),
//                     )
//                     .field(
//                         "state",
//                         Field::new(bevy_input::ButtonState::definition(types)),
//                     )
//                     .field(
//                         "window",
//                         Field::new(bevy_ecs::entity::Entity::definition(types)),
//                     )
//                     .build();
//             }
//         }

//         impl Type for bevy_input::mouse::MouseButtonInput {
//             inline: true;
//             build: |types, ndt| {
//                 ndt.ty = Struct::named()
//                     .field(
//                         "button",
//                         Field::new(bevy_input::mouse::MouseButton::definition(types)),
//                     )
//                     .field(
//                         "state",
//                         Field::new(bevy_input::ButtonState::definition(types)),
//                     )
//                     .field(
//                         "window",
//                         Field::new(bevy_ecs::entity::Entity::definition(types)),
//                     )
//                     .build();
//             }
//         }

//         impl Type for bevy_input::mouse::MouseButton {
//             inline: true;
//             build: |types, ndt| {
//                 ndt.ty = DataType::Enum(Enum {
//                     variants: vec![
//                         ("Left".into(), Variant::unit()),
//                         ("Right".into(), Variant::unit()),
//                         ("Middle".into(), Variant::unit()),
//                         ("Back".into(), Variant::unit()),
//                         ("Forward".into(), Variant::unit()),
//                         (
//                             "Other".into(),
//                             Variant::unnamed()
//                                 .field(Field::new(u16::definition(types)))
//                                 .build(),
//                         ),
//                     ],
//                     attributes: datatype::Attributes::default(),
//                 });
//             }
//         }

//         impl Type for bevy_input::mouse::MouseWheel {
//             inline: true;
//             build: |types, ndt| {
//                 ndt.ty = Struct::named()
//                     .field(
//                         "unit",
//                         Field::new(bevy_input::mouse::MouseScrollUnit::definition(types)),
//                     )
//                     .field("x", Field::new(f32::definition(types)))
//                     .field("y", Field::new(f32::definition(types)))
//                     .field(
//                         "window",
//                         Field::new(bevy_ecs::entity::Entity::definition(types)),
//                     )
//                     .build();
//             }
//         }

//         impl Type for bevy_input::mouse::MouseScrollUnit {
//             inline: true;
//             build: |_types, ndt| {
//                 ndt.ty = DataType::Enum(Enum {
//                     variants: vec![
//                         ("Line".into(), Variant::unit()),
//                         ("Pixel".into(), Variant::unit()),
//                     ],
//                     attributes: datatype::Attributes::default(),
//                 });
//             }
//         }

//         impl Type for bevy_input::mouse::MouseMotion {
//             inline: true;
//             build: |types, ndt| {
//                 ndt.ty = Struct::named()
//                     .field("delta", Field::new(glam::Vec2::definition(types)))
//                     .build();
//             }
//         }
//     );
// };

// #[cfg(feature = "camino")]
// #[cfg_attr(docsrs, doc(cfg(feature = "camino")))]
// impl_ndt!(
//     camino::Utf8Path as str
//     camino::Utf8PathBuf as str
// );

// #[cfg(feature = "geojson")]
// #[cfg_attr(docsrs, doc(cfg(feature = "geojson")))]
// impl_ndt!(geojson::Position as [f64]);

// #[cfg(feature = "geojson")]
// #[cfg_attr(docsrs, doc(cfg(feature = "geojson")))]
// impl_ndt!(
//     impl Type for geojson::GeometryValue {
//         inline: true;
//         build: |types, ndt| {
//             ndt.ty = DataType::Enum(Enum {
//                 variants: vec![
//                     (
//                         "Point".into(),
//                         Variant::unnamed()
//                             .field(Field::new(geojson::PointType::definition(types)))
//                             .build(),
//                     ),
//                     (
//                         "MultiPoint".into(),
//                         Variant::unnamed()
//                             .field(Field::new(Vec::<geojson::PointType>::definition(types)))
//                             .build(),
//                     ),
//                     (
//                         "LineString".into(),
//                         Variant::unnamed()
//                             .field(Field::new(geojson::LineStringType::definition(types)))
//                             .build(),
//                     ),
//                     (
//                         "MultiLineString".into(),
//                         Variant::unnamed()
//                             .field(Field::new(Vec::<geojson::LineStringType>::definition(
//                                 types,
//                             )))
//                             .build(),
//                     ),
//                     (
//                         "Polygon".into(),
//                         Variant::unnamed()
//                             .field(Field::new(geojson::PolygonType::definition(types)))
//                             .build(),
//                     ),
//                     (
//                         "MultiPolygon".into(),
//                         Variant::unnamed()
//                             .field(Field::new(Vec::<geojson::PolygonType>::definition(types)))
//                             .build(),
//                     ),
//                     (
//                         "GeometryCollection".into(),
//                         Variant::unnamed()
//                             .field(Field::new(Vec::<geojson::Geometry>::definition(types)))
//                             .build(),
//                     ),
//                 ],
//                 attributes: datatype::Attributes::default(),
//             });
//         }
//     }

//     impl Type for geojson::Geometry {
//         inline: true;
//         build: |types, ndt| {
//             ndt.ty = Struct::named()
//                 .field("bbox", Field::new(Option::<geojson::Bbox>::definition(types)))
//                 .field("value", Field::new(geojson::GeometryValue::definition(types)))
//                 .field(
//                     "foreign_members",
//                     Field::new(Option::<geojson::JsonObject>::definition(types)),
//                 )
//                 .build();
//         }
//     }

//     impl Type for geojson::Feature {
//         inline: true;
//         build: |types, ndt| {
//             ndt.ty = Struct::named()
//                 .field("bbox", Field::new(Option::<geojson::Bbox>::definition(types)))
//                 .field(
//                     "geometry",
//                     Field::new(Option::<geojson::Geometry>::definition(types)),
//                 )
//                 .field(
//                     "id",
//                     Field::new(Option::<geojson::feature::Id>::definition(types)),
//                 )
//                 .field(
//                     "properties",
//                     Field::new(Option::<geojson::JsonObject>::definition(types)),
//                 )
//                 .field(
//                     "foreign_members",
//                     Field::new(Option::<geojson::JsonObject>::definition(types)),
//                 )
//                 .build();
//         }
//     }

//     impl Type for geojson::FeatureCollection {
//         inline: true;
//         build: |types, ndt| {
//             ndt.ty = Struct::named()
//                 .field("bbox", Field::new(Option::<geojson::Bbox>::definition(types)))
//                 .field(
//                     "features",
//                     Field::new(Vec::<geojson::Feature>::definition(types)),
//                 )
//                 .field(
//                     "foreign_members",
//                     Field::new(Option::<geojson::JsonObject>::definition(types)),
//                 )
//                 .build();
//         }
//     }

//     impl Type for geojson::feature::Id {
//         inline: true;
//         build: |types, ndt| {
//             ndt.ty = DataType::Enum(Enum {
//                 variants: vec![
//                     (
//                         "String".into(),
//                         Variant::unnamed()
//                             .field(Field::new(str::definition(types)))
//                             .build(),
//                     ),
//                     (
//                         "Number".into(),
//                         Variant::unnamed()
//                             .field(Field::new(serde_json::Number::definition(types)))
//                             .build(),
//                     ),
//                 ],
//                 attributes: datatype::Attributes::default(),
//             });
//         }
//     }
// );

// #[cfg(feature = "geozero")]
// #[cfg_attr(docsrs, doc(cfg(feature = "geozero")))]
// impl_ndt!(
//     impl Type for geozero::mvt::Tile {
//         inline: true;
//         build: |types, ndt| {
//             ndt.ty = Struct::named()
//                 .field(
//                     "layers",
//                     Field::new(Vec::<geozero::mvt::tile::Layer>::definition(types)),
//                 )
//                 .build();
//         }
//     }

//     impl Type for geozero::mvt::tile::Value {
//         inline: true;
//         build: |types, ndt| {
//             ndt.ty = Struct::named()
//                 .field("string_value", Field::new(Option::<str>::definition(types)))
//                 .field("float_value", Field::new(Option::<f32>::definition(types)))
//                 .field("double_value", Field::new(Option::<f64>::definition(types)))
//                 .field("int_value", Field::new(Option::<i64>::definition(types)))
//                 .field("uint_value", Field::new(Option::<u64>::definition(types)))
//                 .field("sint_value", Field::new(Option::<i64>::definition(types)))
//                 .field("bool_value", Field::new(Option::<bool>::definition(types)))
//                 .build();
//         }
//     }

//     impl Type for geozero::mvt::tile::Feature {
//         inline: true;
//         build: |types, ndt| {
//             ndt.ty = Struct::named()
//                 .field("id", Field::new(Option::<u64>::definition(types)))
//                 .field("tags", Field::new(Vec::<u32>::definition(types)))
//                 .field("type", Field::new(Option::<i32>::definition(types)))
//                 .field("geometry", Field::new(Vec::<u32>::definition(types)))
//                 .build();
//         }
//     }

//     impl Type for geozero::mvt::tile::Layer {
//         inline: true;
//         build: |types, ndt| {
//             ndt.ty = Struct::named()
//                 .field("version", Field::new(u32::definition(types)))
//                 .field("name", Field::new(str::definition(types)))
//                 .field(
//                     "features",
//                     Field::new(Vec::<geozero::mvt::tile::Feature>::definition(types)),
//                 )
//                 .field("keys", Field::new(Vec::<str>::definition(types)))
//                 .field(
//                     "values",
//                     Field::new(Vec::<geozero::mvt::tile::Value>::definition(types)),
//                 )
//                 .field("extent", Field::new(Option::<u32>::definition(types)))
//                 .build();
//         }
//     }

//     impl Type for geozero::mvt::tile::GeomType {
//         inline: true;
//         build: |_types, ndt| {
//             ndt.ty = DataType::Enum(Enum {
//                 variants: vec![
//                     ("Unknown".into(), Variant::unit()),
//                     ("Point".into(), Variant::unit()),
//                     ("Linestring".into(), Variant::unit()),
//                     ("Polygon".into(), Variant::unit()),
//                 ],
//                 attributes: datatype::Attributes::default(),
//             });
//         }
//     }
// );
