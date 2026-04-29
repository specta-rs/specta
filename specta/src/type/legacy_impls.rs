use crate::{Type, Types};
#[allow(unused_imports)]
use crate::{datatype::*, r#type::impls::*, r#type::macros::impl_ndt};

// `String` requires `std` feature, while `str` does not.
// We can't use `str` in a lot of places because it's not `Sized`, so this bridges that.
#[allow(unused)]
pub struct String;
impl Type for String {
    fn definition(types: &mut Types) -> DataType {
        str::definition(types)
    }
}

#[cfg(feature = "indexmap")]
#[cfg_attr(docsrs, doc(cfg(feature = "indexmap")))]
const _: () = {
    impl_ndt!(
        indexmap::IndexSet<T> as PrimitiveSet<T> = inline_passthrough;
        indexmap::IndexMap<K, V> as PrimitiveMap<K, V> = inline_passthrough;
    );
};

#[cfg(feature = "ordered-float")]
#[cfg_attr(docsrs, doc(cfg(feature = "ordered-float")))]
impl_ndt!(
    ordered_float::OrderedFloat<T> where { T: Type + ordered_float::FloatCore } as T = inline;
    ordered_float::NotNan<T> where { T: Type + ordered_float::FloatCore } as T = inline;
);

#[cfg(feature = "heapless")]
#[cfg_attr(docsrs, doc(cfg(feature = "heapless")))]
const _: () = {
    impl_ndt!(
        // Sequential containers
        heapless::Vec<T> <T, const N: usize, LenT> where { T: Type, LenT: heapless::LenType } as [T; N] = inline_passthrough;
        heapless::Deque<T> <T, const N: usize> where { T: Type } as [T; N] = inline_passthrough;
        heapless::HistoryBuf<T> <T, const N: usize> where { T: Type } as [T; N] = inline_passthrough;
        heapless::BinaryHeap<T, K> <T, K, const N: usize> where { T: Type + Ord, K: heapless::binary_heap::Kind } as [T; N] = inline_passthrough;

        // Sets
        heapless::IndexSet<T, S> <T, S, const N: usize> where { T: Type + Eq + core::hash::Hash, S: core::hash::BuildHasher } as PrimitiveSet<T> = inline_passthrough;

        // Maps
        heapless::IndexMap<K, V, S> <K, V, S, const N: usize> where { K: Type + Eq + core::hash::Hash, V: Type, S: core::hash::BuildHasher } as PrimitiveMap<K, V> = inline_passthrough;
        heapless::LinearMap<K, V> <K, V, const N: usize> where { K: Type + Eq, V: Type } as PrimitiveMap<K, V> = inline_passthrough;

        // String container
        heapless::String <const N: usize, LenT> where { LenT: heapless::LenType } as str = inline;
    );
};

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
impl_ndt!(smallvec::SmallVec<T> where { T: smallvec::Array + Type } as T = inline_passthrough);

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
                attributes: Attributes::default(),
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
                attributes: Attributes::default(),
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
                attributes: Attributes::default(),
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
                attributes: Attributes::default(),
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
    time::PrimitiveDateTime as str = inline;
    time::OffsetDateTime as str = inline;
    time::Date as str = inline;
    time::UtcDateTime as str = inline;
    time::Time as str = inline;
    time::Duration as str = inline;
    time::UtcOffset as str = inline;
    time::Weekday as str = inline;
    time::Month as str = inline;
);

#[cfg(feature = "jiff")]
#[cfg_attr(docsrs, doc(cfg(feature = "jiff")))]
impl_ndt!(
    jiff::Timestamp as str = inline;
    jiff::Zoned as str = inline;
    jiff::SignedDuration as str = inline;
    jiff::civil::Date as str = inline;
    jiff::civil::Time as str = inline;
    jiff::civil::DateTime as str = inline;
    jiff::civil::ISOWeekDate as str = inline;
    jiff::tz::TimeZone as str = inline;
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
        bson::Utf8Lossy<T> as T = inline_passthrough;
        bson::RawBsonRef<'a,> as Bson = inline;
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
                attributes: Attributes::default(),
            })
        }
    }
};

// Technically this can be u64 for formats not marked as human readable in Serde.
// but we have no way of inspecting that as it's runtime. This is the most common output.
#[cfg(feature = "bytesize")]
#[cfg_attr(docsrs, doc(cfg(feature = "bytesize")))]
impl_ndt!(bytesize::ByteSize as String = inline);

#[cfg(feature = "uhlc")]
#[cfg_attr(docsrs, doc(cfg(feature = "uhlc")))]
const _: () = {
    use crate::{datatype, r#type::macros::impl_ndt, *};

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
                                docs: Default::default(),
                                ty: Some(uhlc::NTP64::definition(types)),
                                attributes: Attributes::default(),
                            },
                        ),
                        (
                            "id".into(),
                            Field {
                                optional: false,
                                deprecated: None,
                                docs: Default::default(),
                                ty: Some(uhlc::ID::definition(types)),
                                attributes: Attributes::default(),
                            },
                        ),
                    ],
                }),
                attributes: Attributes::default(),
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
    use crate::{datatype, r#type::macros::impl_ndt, *};

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
                attributes: Attributes::default(),
            })
        }
    }
};

#[cfg(feature = "either")]
#[cfg_attr(docsrs, doc(cfg(feature = "either")))]
const _: () = {
    use crate::{datatype, r#type::macros::impl_ndt, *};

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
                attributes: Attributes::default(),
            })
        }
    }
};

#[cfg(feature = "error-stack")]
#[cfg_attr(docsrs, doc(cfg(feature = "error-stack")))]
const _: () = {
    use crate::r#type::impls::*;
    use crate::r#type::macros::impl_ndt;

    impl_ndt!(
        "error_stack" ErrorStackContext as ErrorStackContextInner = named;
        error_stack::Report<C> where { C: std::error::Error + Send + Sync + 'static } as ReportInner = named;
    );

    impl<C: std::error::Error + Send + Sync + 'static> Type for error_stack::Report<[C]> {
        fn definition(types: &mut Types) -> DataType {
            error_stack::Report::<C>::definition(types)
        }
    }

    struct ErrorStackContext;

    struct ErrorStackContextInner;
    impl Type for ErrorStackContextInner {
        fn definition(types: &mut Types) -> DataType {
            let attachments = DataType::List(List::new(String::definition(types)));
            let sources = DataType::List(List::new(ErrorStackContext::definition(types)));

            Struct::named()
                .field("context", Field::new(String::definition(types)))
                .field("attachments", Field::new(attachments))
                .field("sources", Field::new(sources))
                .build()
        }
    }

    struct ReportInner;
    impl Type for ReportInner {
        fn definition(types: &mut Types) -> DataType {
            DataType::List(List::new(ErrorStackContext::definition(types)))
        }
    }
};

#[cfg(feature = "bevy_ecs")]
#[cfg_attr(docsrs, doc(cfg(feature = "bevy_ecs")))]
const _: () = {
    use crate::r#type::impls::*;
    use crate::r#type::macros::impl_ndt;

    impl_ndt!(
        bevy_ecs::entity::Entity as u64 = named;
        bevy_ecs::name::Name as str = named;
        bevy_ecs::hierarchy::ChildOf as bevy_ecs::entity::Entity = named;
        bevy_ecs::entity::EntityHashMap<V> where { V: Type } as PrimitiveMap<bevy_ecs::entity::Entity, V> = named;
        bevy_ecs::entity::EntityHashSet as PrimitiveSet<bevy_ecs::entity::Entity> = named;
        bevy_ecs::entity::EntityIndexMap<V> where { V: Type } as PrimitiveMap<bevy_ecs::entity::Entity, V> = named;
        bevy_ecs::entity::EntityIndexSet as PrimitiveSet<bevy_ecs::entity::Entity> = named;
    );
};

#[cfg(feature = "bevy_input")]
#[cfg_attr(docsrs, doc(cfg(feature = "bevy_input")))]
const _: () = {
    use crate::{datatype, r#type::macros::impl_ndt, *};

    impl_ndt!(
        bevy_input::ButtonState as BevyButtonState = named;
        bevy_input::keyboard::KeyboardInput as BevyKeyboardInput = named;
        bevy_input::keyboard::KeyboardFocusLost as BevyKeyboardFocusLost = named;
        bevy_input::keyboard::NativeKeyCode as str = named;
        bevy_input::keyboard::KeyCode as str = named;
        bevy_input::keyboard::NativeKey as str = named;
        bevy_input::keyboard::Key as str = named;
        bevy_input::mouse::MouseButtonInput as BevyMouseButtonInput = named;
        bevy_input::mouse::MouseButton as BevyMouseButton = named;
        bevy_input::mouse::MouseMotion as BevyMouseMotion = named;
        bevy_input::mouse::MouseScrollUnit as BevyMouseScrollUnit = named;
        bevy_input::mouse::MouseWheel as BevyMouseWheel = named;
        bevy_input::mouse::AccumulatedMouseMotion as BevyAccumulatedMouseMotion = named;
        bevy_input::mouse::AccumulatedMouseScroll as BevyAccumulatedMouseScroll = named;
        bevy_input::touch::TouchInput as BevyTouchInput = named;
        bevy_input::touch::ForceTouch as BevyForceTouch = named;
        bevy_input::touch::TouchPhase as BevyTouchPhase = named;
        bevy_input::gestures::PinchGesture as f32 = named;
        bevy_input::gestures::RotationGesture as f32 = named;
        bevy_input::gestures::DoubleTapGesture as BevyDoubleTapGesture = named;
        bevy_input::gestures::PanGesture as [f32; 2] = named;
        bevy_input::gamepad::GamepadEvent as BevyGamepadEvent = named;
        bevy_input::gamepad::RawGamepadEvent as BevyRawGamepadEvent = named;
        bevy_input::gamepad::RawGamepadButtonChangedEvent as BevyRawGamepadButtonChangedEvent = named;
        bevy_input::gamepad::RawGamepadAxisChangedEvent as BevyRawGamepadAxisChangedEvent = named;
        bevy_input::gamepad::GamepadConnectionEvent as BevyGamepadConnectionEvent = named;
        bevy_input::gamepad::GamepadButtonStateChangedEvent as BevyGamepadButtonStateChangedEvent = named;
        bevy_input::gamepad::GamepadButtonChangedEvent as BevyGamepadButtonChangedEvent = named;
        bevy_input::gamepad::GamepadAxisChangedEvent as BevyGamepadAxisChangedEvent = named;
        bevy_input::gamepad::GamepadButton as BevyGamepadButton = named;
        bevy_input::gamepad::GamepadAxis as BevyGamepadAxis = named;
        bevy_input::gamepad::GamepadConnection as BevyGamepadConnection = named;
    );

    struct BevyButtonState;
    impl Type for BevyButtonState {
        fn definition(_: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    ("Pressed".into(), Variant::unit()),
                    ("Released".into(), Variant::unit()),
                ],
                attributes: Attributes::default(),
            })
        }
    }

    struct BevyKeyboardFocusLost;
    impl Type for BevyKeyboardFocusLost {
        fn definition(_: &mut Types) -> DataType {
            DataType::Struct(Struct {
                fields: Fields::Unit,
                attributes: Attributes::default(),
            })
        }
    }

    struct BevyKeyboardInput;
    impl Type for BevyKeyboardInput {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "key_code",
                    Field::new(bevy_input::keyboard::KeyCode::definition(types)),
                )
                .field(
                    "logical_key",
                    Field::new(bevy_input::keyboard::Key::definition(types)),
                )
                .field(
                    "state",
                    Field::new(bevy_input::ButtonState::definition(types)),
                )
                .field(
                    "text",
                    Field::new(Option::<smol_str::SmolStr>::definition(types)),
                )
                .field("repeat", Field::new(bool::definition(types)))
                .field(
                    "window",
                    Field::new(bevy_ecs::entity::Entity::definition(types)),
                )
                .build()
        }
    }

    struct BevyMouseButtonInput;
    impl Type for BevyMouseButtonInput {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "button",
                    Field::new(bevy_input::mouse::MouseButton::definition(types)),
                )
                .field(
                    "state",
                    Field::new(bevy_input::ButtonState::definition(types)),
                )
                .field(
                    "window",
                    Field::new(bevy_ecs::entity::Entity::definition(types)),
                )
                .build()
        }
    }

    struct BevyMouseButton;
    impl Type for BevyMouseButton {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
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
                attributes: Attributes::default(),
            })
        }
    }

    struct BevyMouseMotion;
    impl Type for BevyMouseMotion {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field("delta", Field::new(<[f32; 2]>::definition(types)))
                .build()
        }
    }

    struct BevyMouseScrollUnit;
    impl Type for BevyMouseScrollUnit {
        fn definition(_: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    ("Line".into(), Variant::unit()),
                    ("Pixel".into(), Variant::unit()),
                ],
                attributes: Attributes::default(),
            })
        }
    }

    struct BevyMouseWheel;
    impl Type for BevyMouseWheel {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "unit",
                    Field::new(bevy_input::mouse::MouseScrollUnit::definition(types)),
                )
                .field("x", Field::new(f32::definition(types)))
                .field("y", Field::new(f32::definition(types)))
                .field(
                    "window",
                    Field::new(bevy_ecs::entity::Entity::definition(types)),
                )
                .field(
                    "phase",
                    Field::new(bevy_input::touch::TouchPhase::definition(types)),
                )
                .build()
        }
    }

    struct BevyAccumulatedMouseMotion;
    impl Type for BevyAccumulatedMouseMotion {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field("delta", Field::new(<[f32; 2]>::definition(types)))
                .build()
        }
    }

    struct BevyAccumulatedMouseScroll;
    impl Type for BevyAccumulatedMouseScroll {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "unit",
                    Field::new(bevy_input::mouse::MouseScrollUnit::definition(types)),
                )
                .field("delta", Field::new(<[f32; 2]>::definition(types)))
                .build()
        }
    }

    struct BevyTouchInput;
    impl Type for BevyTouchInput {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "phase",
                    Field::new(bevy_input::touch::TouchPhase::definition(types)),
                )
                .field("position", Field::new(<[f32; 2]>::definition(types)))
                .field(
                    "window",
                    Field::new(bevy_ecs::entity::Entity::definition(types)),
                )
                .field(
                    "force",
                    Field::new(Option::<bevy_input::touch::ForceTouch>::definition(types)),
                )
                .field("id", Field::new(u64::definition(types)))
                .build()
        }
    }

    struct BevyForceTouch;
    impl Type for BevyForceTouch {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    (
                        "Calibrated".into(),
                        Variant::named()
                            .field("force", Field::new(f64::definition(types)))
                            .field("max_possible_force", Field::new(f64::definition(types)))
                            .field(
                                "altitude_angle",
                                Field::new(Option::<f64>::definition(types)),
                            )
                            .build(),
                    ),
                    (
                        "Normalized".into(),
                        Variant::unnamed()
                            .field(Field::new(f64::definition(types)))
                            .build(),
                    ),
                ],
                attributes: Attributes::default(),
            })
        }
    }

    struct BevyTouchPhase;
    impl Type for BevyTouchPhase {
        fn definition(_: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    ("Started".into(), Variant::unit()),
                    ("Moved".into(), Variant::unit()),
                    ("Ended".into(), Variant::unit()),
                    ("Canceled".into(), Variant::unit()),
                ],
                attributes: Attributes::default(),
            })
        }
    }

    struct BevyDoubleTapGesture;
    impl Type for BevyDoubleTapGesture {
        fn definition(_: &mut Types) -> DataType {
            DataType::Struct(Struct {
                fields: Fields::Unit,
                attributes: Attributes::default(),
            })
        }
    }

    struct BevyGamepadEvent;
    impl Type for BevyGamepadEvent {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    (
                        "Connection".into(),
                        Variant::unnamed()
                            .field(Field::new(
                                bevy_input::gamepad::GamepadConnectionEvent::definition(types),
                            ))
                            .build(),
                    ),
                    (
                        "Button".into(),
                        Variant::unnamed()
                            .field(Field::new(
                                bevy_input::gamepad::GamepadButtonChangedEvent::definition(types),
                            ))
                            .build(),
                    ),
                    (
                        "Axis".into(),
                        Variant::unnamed()
                            .field(Field::new(
                                bevy_input::gamepad::GamepadAxisChangedEvent::definition(types),
                            ))
                            .build(),
                    ),
                ],
                attributes: Attributes::default(),
            })
        }
    }

    struct BevyRawGamepadEvent;
    impl Type for BevyRawGamepadEvent {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    (
                        "Connection".into(),
                        Variant::unnamed()
                            .field(Field::new(
                                bevy_input::gamepad::GamepadConnectionEvent::definition(types),
                            ))
                            .build(),
                    ),
                    (
                        "Button".into(),
                        Variant::unnamed()
                            .field(Field::new(
                                bevy_input::gamepad::RawGamepadButtonChangedEvent::definition(
                                    types,
                                ),
                            ))
                            .build(),
                    ),
                    (
                        "Axis".into(),
                        Variant::unnamed()
                            .field(Field::new(
                                bevy_input::gamepad::RawGamepadAxisChangedEvent::definition(types),
                            ))
                            .build(),
                    ),
                ],
                attributes: Attributes::default(),
            })
        }
    }

    struct BevyRawGamepadButtonChangedEvent;
    impl Type for BevyRawGamepadButtonChangedEvent {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "gamepad",
                    Field::new(bevy_ecs::entity::Entity::definition(types)),
                )
                .field(
                    "button",
                    Field::new(bevy_input::gamepad::GamepadButton::definition(types)),
                )
                .field("value", Field::new(f32::definition(types)))
                .build()
        }
    }

    struct BevyRawGamepadAxisChangedEvent;
    impl Type for BevyRawGamepadAxisChangedEvent {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "gamepad",
                    Field::new(bevy_ecs::entity::Entity::definition(types)),
                )
                .field(
                    "axis",
                    Field::new(bevy_input::gamepad::GamepadAxis::definition(types)),
                )
                .field("value", Field::new(f32::definition(types)))
                .build()
        }
    }

    struct BevyGamepadConnectionEvent;
    impl Type for BevyGamepadConnectionEvent {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "gamepad",
                    Field::new(bevy_ecs::entity::Entity::definition(types)),
                )
                .field(
                    "connection",
                    Field::new(bevy_input::gamepad::GamepadConnection::definition(types)),
                )
                .build()
        }
    }

    struct BevyGamepadButtonStateChangedEvent;
    impl Type for BevyGamepadButtonStateChangedEvent {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "entity",
                    Field::new(bevy_ecs::entity::Entity::definition(types)),
                )
                .field(
                    "button",
                    Field::new(bevy_input::gamepad::GamepadButton::definition(types)),
                )
                .field(
                    "state",
                    Field::new(bevy_input::ButtonState::definition(types)),
                )
                .build()
        }
    }

    struct BevyGamepadButtonChangedEvent;
    impl Type for BevyGamepadButtonChangedEvent {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "entity",
                    Field::new(bevy_ecs::entity::Entity::definition(types)),
                )
                .field(
                    "button",
                    Field::new(bevy_input::gamepad::GamepadButton::definition(types)),
                )
                .field(
                    "state",
                    Field::new(bevy_input::ButtonState::definition(types)),
                )
                .field("value", Field::new(f32::definition(types)))
                .build()
        }
    }

    struct BevyGamepadAxisChangedEvent;
    impl Type for BevyGamepadAxisChangedEvent {
        fn definition(types: &mut Types) -> DataType {
            Struct::named()
                .field(
                    "entity",
                    Field::new(bevy_ecs::entity::Entity::definition(types)),
                )
                .field(
                    "axis",
                    Field::new(bevy_input::gamepad::GamepadAxis::definition(types)),
                )
                .field("value", Field::new(f32::definition(types)))
                .build()
        }
    }

    struct BevyGamepadButton;
    impl Type for BevyGamepadButton {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    ("South".into(), Variant::unit()),
                    ("East".into(), Variant::unit()),
                    ("North".into(), Variant::unit()),
                    ("West".into(), Variant::unit()),
                    ("C".into(), Variant::unit()),
                    ("Z".into(), Variant::unit()),
                    ("LeftTrigger".into(), Variant::unit()),
                    ("LeftTrigger2".into(), Variant::unit()),
                    ("RightTrigger".into(), Variant::unit()),
                    ("RightTrigger2".into(), Variant::unit()),
                    ("Select".into(), Variant::unit()),
                    ("Start".into(), Variant::unit()),
                    ("Mode".into(), Variant::unit()),
                    ("LeftThumb".into(), Variant::unit()),
                    ("RightThumb".into(), Variant::unit()),
                    ("DPadUp".into(), Variant::unit()),
                    ("DPadDown".into(), Variant::unit()),
                    ("DPadLeft".into(), Variant::unit()),
                    ("DPadRight".into(), Variant::unit()),
                    (
                        "Other".into(),
                        Variant::unnamed()
                            .field(Field::new(u8::definition(types)))
                            .build(),
                    ),
                ],
                attributes: Attributes::default(),
            })
        }
    }

    struct BevyGamepadAxis;
    impl Type for BevyGamepadAxis {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    ("LeftStickX".into(), Variant::unit()),
                    ("LeftStickY".into(), Variant::unit()),
                    ("LeftZ".into(), Variant::unit()),
                    ("RightStickX".into(), Variant::unit()),
                    ("RightStickY".into(), Variant::unit()),
                    ("RightZ".into(), Variant::unit()),
                    (
                        "Other".into(),
                        Variant::unnamed()
                            .field(Field::new(u8::definition(types)))
                            .build(),
                    ),
                ],
                attributes: Attributes::default(),
            })
        }
    }

    struct BevyGamepadConnection;
    impl Type for BevyGamepadConnection {
        fn definition(types: &mut Types) -> DataType {
            DataType::Enum(Enum {
                variants: vec![
                    (
                        "Connected".into(),
                        Variant::named()
                            .field("name", Field::new(String::definition(types)))
                            .field("vendor_id", Field::new(Option::<u16>::definition(types)))
                            .field("product_id", Field::new(Option::<u16>::definition(types)))
                            .build(),
                    ),
                    ("Disconnected".into(), Variant::unit()),
                ],
                attributes: Attributes::default(),
            })
        }
    }
};

#[cfg(feature = "camino")]
#[cfg_attr(docsrs, doc(cfg(feature = "camino")))]
impl_ndt!(
    camino::Utf8Path as str = inline;
    camino::Utf8PathBuf as str = inline;
);

#[cfg(feature = "geojson")]
#[cfg_attr(docsrs, doc(cfg(feature = "geojson")))]
const _: () = {
    use crate::{datatype, r#type::macros::impl_ndt, *};

    impl_ndt!(
        geojson::Position as [f64] = inline;
        geojson::GeoJson as GeoJson = inline;
        geojson::GeometryValue as GeoJsonGeometryValue = inline;
        geojson::Geometry as GeoJsonGeometry = inline;
        geojson::Feature as GeoJsonFeature = inline;
        geojson::FeatureCollection as GeoJsonFeatureCollection = inline;
        geojson::feature::Id as GeoJsonFeatureId = inline;
    );

    struct GeoJson;
    impl Type for GeoJson {
        fn definition(types: &mut Types) -> DataType {
            let mut attributes = Attributes::default();
            attributes.insert("serde:container:untagged", true);

            DataType::Enum(Enum {
                variants: vec![
                    (
                        "Geometry".into(),
                        Variant::unnamed()
                            .field(Field::new(geojson::Geometry::definition(types)))
                            .build(),
                    ),
                    (
                        "Feature".into(),
                        Variant::unnamed()
                            .field(Field::new(geojson::Feature::definition(types)))
                            .build(),
                    ),
                    (
                        "FeatureCollection".into(),
                        Variant::unnamed()
                            .field(Field::new(geojson::FeatureCollection::definition(types)))
                            .build(),
                    ),
                ],
                attributes,
            })
        }
    }

    struct GeoJsonGeometryValue;
    impl Type for GeoJsonGeometryValue {
        fn definition(types: &mut Types) -> DataType {
            let mut attributes = Attributes::default();
            attributes.insert("serde:container:tag", std::string::String::from("type"));

            DataType::Enum(Enum {
                variants: vec![
                    (
                        "Point".into(),
                        Variant::named()
                            .field(
                                "coordinates",
                                Field::new(geojson::PointType::definition(types)),
                            )
                            .build(),
                    ),
                    (
                        "MultiPoint".into(),
                        Variant::named()
                            .field(
                                "coordinates",
                                Field::new(Vec::<geojson::PointType>::definition(types)),
                            )
                            .build(),
                    ),
                    (
                        "LineString".into(),
                        Variant::named()
                            .field(
                                "coordinates",
                                Field::new(geojson::LineStringType::definition(types)),
                            )
                            .build(),
                    ),
                    (
                        "MultiLineString".into(),
                        Variant::named()
                            .field(
                                "coordinates",
                                Field::new(Vec::<geojson::LineStringType>::definition(types)),
                            )
                            .build(),
                    ),
                    (
                        "Polygon".into(),
                        Variant::named()
                            .field(
                                "coordinates",
                                Field::new(geojson::PolygonType::definition(types)),
                            )
                            .build(),
                    ),
                    (
                        "MultiPolygon".into(),
                        Variant::named()
                            .field(
                                "coordinates",
                                Field::new(Vec::<geojson::PolygonType>::definition(types)),
                            )
                            .build(),
                    ),
                    (
                        "GeometryCollection".into(),
                        Variant::named()
                            .field(
                                "geometries",
                                Field::new(Vec::<geojson::Geometry>::definition(types)),
                            )
                            .build(),
                    ),
                ],
                attributes,
            })
        }
    }

    struct GeoJsonGeometry;
    impl Type for GeoJsonGeometry {
        fn definition(types: &mut Types) -> DataType {
            let mut value = Field::new(geojson::GeometryValue::definition(types));
            value.attributes.insert("serde:field:flatten", true);

            let mut foreign_members = Field::new(Option::<geojson::JsonObject>::definition(types));
            foreign_members
                .attributes
                .insert("serde:field:flatten", true);

            Struct::named()
                .field(
                    "bbox",
                    Field::new(Option::<geojson::Bbox>::definition(types)),
                )
                .field("value", value)
                .field("foreign_members", foreign_members)
                .build()
        }
    }

    struct GeoJsonFeature;
    impl Type for GeoJsonFeature {
        fn definition(types: &mut Types) -> DataType {
            let mut attributes = Attributes::default();
            attributes.insert("serde:container:tag", std::string::String::from("type"));

            let mut foreign_members = Field::new(Option::<geojson::JsonObject>::definition(types));
            foreign_members
                .attributes
                .insert("serde:field:flatten", true);

            DataType::Struct(Struct {
                fields: Fields::Named(NamedFields {
                    fields: vec![
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
                        ("foreign_members".into(), foreign_members),
                    ],
                }),
                attributes,
            })
        }
    }

    struct GeoJsonFeatureCollection;
    impl Type for GeoJsonFeatureCollection {
        fn definition(types: &mut Types) -> DataType {
            let mut attributes = Attributes::default();
            attributes.insert("serde:container:tag", std::string::String::from("type"));

            let mut foreign_members = Field::new(Option::<geojson::JsonObject>::definition(types));
            foreign_members
                .attributes
                .insert("serde:field:flatten", true);

            DataType::Struct(Struct {
                fields: Fields::Named(NamedFields {
                    fields: vec![
                        (
                            "bbox".into(),
                            Field::new(Option::<geojson::Bbox>::definition(types)),
                        ),
                        (
                            "features".into(),
                            Field::new(Vec::<geojson::Feature>::definition(types)),
                        ),
                        ("foreign_members".into(), foreign_members),
                    ],
                }),
                attributes,
            })
        }
    }

    struct GeoJsonFeatureId;
    impl Type for GeoJsonFeatureId {
        fn definition(types: &mut Types) -> DataType {
            let mut attributes = Attributes::default();
            attributes.insert("serde:container:untagged", true);

            DataType::Enum(Enum {
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
                attributes,
            })
        }
    }
};
