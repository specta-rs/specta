use crate::*;

impl_primitives!(
    i8 i16 i32 i64 i128 isize
    u8 u16 u32 u64 u128 usize
    f32 f64
    bool char
    String
);

impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

const _: () = {
    use std::{cell::*, rc::Rc, sync::*};
    impl_containers!(Box Rc Arc Cell RefCell Mutex RwLock);
};

#[cfg(feature = "tokio")]
const _: () = {
    use tokio::sync::{Mutex, RwLock};
    impl_containers!(Mutex RwLock);
};

impl<'a> Type for &'a str {
    fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
        String::inline(opts, generics)
    }
}

impl<'a, T: Type + 'static> Type for &'a T {
    fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
        T::inline(opts, generics)
    }
}

impl<T: Type> Type for [T] {
    fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
        T::inline(opts, generics)
    }
}

impl<'a, T: ?Sized + ToOwned + Type + 'static> Type for std::borrow::Cow<'a, T> {
    fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
        T::inline(opts, generics)
    }
}

use std::ffi::*;
impl_as!(
    str as String
    CString as String
    CStr as String
    OsString as String
    OsStr as String
);

use std::path::*;
impl_as!(
    Path as String
    PathBuf as String
);

use std::net::*;
impl_as!(
    IpAddr as String
    Ipv4Addr as String
    Ipv6Addr as String

    SocketAddr as String
    SocketAddrV4 as String
    SocketAddrV6 as String
);

use std::sync::atomic::*;
impl_as!(
    AtomicBool as bool
    AtomicI8 as i8
    AtomicI16 as i16
    AtomicI32 as i32
    AtomicIsize as isize
    AtomicU8 as u8
    AtomicU16 as u16
    AtomicU32 as u32
    AtomicUsize as usize
    AtomicI64 as i64
    AtomicU64 as u64
);

use std::num::*;
impl_as!(
    NonZeroU8 as u8
    NonZeroU16 as u16
    NonZeroU32 as u32
    NonZeroU64 as u64
    NonZeroUsize as usize
    NonZeroI8 as i8
    NonZeroI16 as i16
    NonZeroI32 as i32
    NonZeroI64 as i64
    NonZeroIsize as isize
    NonZeroU128 as u128
    NonZeroI128 as i128
);

use std::collections::*;
impl_for_list!(
    Vec<T> as "Vec"
    VecDeque<T> as "VecDeque"
    BinaryHeap<T> as "BinaryHeap"
    LinkedList<T> as "LinkedList"
    HashSet<T> as "HashSet"
    BTreeSet<T> as "BTreeSet"
);

impl<'a, T: Type> Type for &'a [T] {
    fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
        <Vec<T>>::inline(opts, generics)
    }

    fn category_impl(opts: DefOpts, generics: &[DataType]) -> Result<TypeCategory, ExportError> {
        <Vec<T>>::category_impl(opts, generics)
    }
}

impl<const N: usize, T: Type> Type for [T; N] {
    fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
        <Vec<T>>::inline(opts, generics)
    }

    fn category_impl(opts: DefOpts, generics: &[DataType]) -> Result<TypeCategory, ExportError> {
        <Vec<T>>::category_impl(opts, generics)
    }
}

impl<T: Type> Type for Option<T> {
    fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
        Ok(DataType::Nullable(Box::new(
            generics
                .get(0)
                .cloned()
                .map_or_else(|| T::inline(opts, generics), Ok)?,
        )))
    }

    fn category_impl(opts: DefOpts, generics: &[DataType]) -> Result<TypeCategory, ExportError> {
        Ok(TypeCategory::Inline(DataType::Nullable(Box::new(
            generics
                .get(0)
                .cloned()
                .map_or_else(|| T::reference(opts, generics), Ok)?,
        ))))
    }
}

impl<T: Type, E: Type> Type for Result<T, E> {
    fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
        Ok(DataType::Result(Box::new((
            T::inline(
                DefOpts {
                    parent_inline: opts.parent_inline,
                    type_map: opts.type_map,
                },
                generics,
            )?,
            E::inline(
                DefOpts {
                    parent_inline: opts.parent_inline,
                    type_map: opts.type_map,
                },
                generics,
            )?,
        ))))
    }
}

impl<T> Type for std::marker::PhantomData<T> {
    fn inline(_: DefOpts, _: &[DataType]) -> Result<DataType, ExportError> {
        Ok(DataType::Literal(LiteralType::None))
    }
}

#[allow(unused)]
#[derive(Type)]
#[specta(remote = std::convert::Infallible, crate = crate)]
pub enum Infallible {}

impl<T: Type> Type for std::ops::Range<T> {
    fn inline(opts: DefOpts, _generics: &[DataType]) -> Result<DataType, ExportError> {
        let ty = T::definition(opts)?;
        Ok(DataType::Struct(StructType::Named(StructNamedFields {
            generics: vec![],
            fields: vec![
                StructField {
                    key: "start".into(),
                    optional: false,
                    flatten: false,
                    ty: ty.clone(),
                },
                StructField {
                    key: "end".into(),
                    optional: false,
                    flatten: false,
                    ty,
                },
            ],
            tag: None,
        })))
    }
}

impl<T: Type> Type for std::ops::RangeInclusive<T> {
    fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
        std::ops::Range::<T>::inline(opts, generics) // Yeah Serde are cringe
    }
}

impl_for_map!(HashMap<K, V> as "HashMap");
impl_for_map!(BTreeMap<K, V> as "BTreeMap");
impl<K: Type, V: Type> Flatten for std::collections::HashMap<K, V> {}
impl<K: Type, V: Type> Flatten for std::collections::BTreeMap<K, V> {}

#[derive(Type)]
#[specta(remote = std::time::SystemTime, crate = crate, export = false)]
#[allow(dead_code)]
struct SystemTime {
    duration_since_epoch: i64,
    duration_since_unix_epoch: u32,
}

#[derive(Type)]
#[specta(remote = std::time::Duration, crate = crate, export = false)]
#[allow(dead_code)]
struct Duration {
    secs: u64,
    nanos: u32,
}

#[cfg(feature = "indexmap")]
const _: () = {
    impl_for_list!(indexmap::IndexSet<T> as "IndexSet");
    impl_for_map!(indexmap::IndexMap<K, V> as "IndexMap");
    impl<K: Type, V: Type> Flatten for indexmap::IndexMap<K, V> {}
};

#[cfg(feature = "serde_json")]
const _: () = {
    impl_for_map!(serde_json::Map<K, V> as "Map");
    impl<K: Type, V: Type> Flatten for serde_json::Map<K, V> {}

    impl Type for serde_json::Value {
        fn inline(_: DefOpts, _: &[DataType]) -> Result<DataType, ExportError> {
            Ok(DataType::Any)
        }
    }

    impl Type for serde_json::Number {
        fn inline(_: DefOpts, _: &[DataType]) -> Result<DataType, ExportError> {
            Ok(DataType::Enum(
                UntaggedEnum {
                    variants: vec![
                        EnumVariant::Unnamed(TupleType {
                            fields: vec![DataType::Primitive(PrimitiveType::f64)],
                            generics: vec![],
                        }),
                        EnumVariant::Unnamed(TupleType {
                            fields: vec![DataType::Primitive(PrimitiveType::i64)],
                            generics: vec![],
                        }),
                        EnumVariant::Unnamed(TupleType {
                            fields: vec![DataType::Primitive(PrimitiveType::u64)],
                            generics: vec![],
                        }),
                    ],
                    generics: vec![],
                }
                .into(),
            ))
        }
    }
};

#[cfg(feature = "serde_yaml")]
const _: () = {
    impl Type for serde_yaml::Value {
        fn inline(_: DefOpts, _: &[DataType]) -> Result<DataType, ExportError> {
            Ok(DataType::Any)
        }
    }

    impl Type for serde_yaml::Mapping {
        fn inline(_: DefOpts, _: &[DataType]) -> Result<DataType, ExportError> {
            Ok(DataType::Any)
        }
    }

    impl Type for serde_yaml::value::TaggedValue {
        fn inline(_: DefOpts, _: &[DataType]) -> Result<DataType, ExportError> {
            Ok(DataType::Any)
        }
    }

    impl Type for serde_yaml::Number {
        fn inline(_: DefOpts, _: &[DataType]) -> Result<DataType, ExportError> {
            Ok(DataType::Enum(
                UntaggedEnum {
                    variants: vec![
                        EnumVariant::Unnamed(TupleType {
                            fields: vec![DataType::Primitive(PrimitiveType::f64)],
                            generics: vec![],
                        }),
                        EnumVariant::Unnamed(TupleType {
                            fields: vec![DataType::Primitive(PrimitiveType::i64)],
                            generics: vec![],
                        }),
                        EnumVariant::Unnamed(TupleType {
                            fields: vec![DataType::Primitive(PrimitiveType::u64)],
                            generics: vec![],
                        }),
                    ],
                    generics: vec![],
                }
                .into(),
            ))
        }
    }
};

#[cfg(feature = "toml")]
const _: () = {
    impl_for_map!(toml::map::Map<K, V> as "Map");
    impl<K: Type, V: Type> Flatten for toml::map::Map<K, V> {}

    impl Type for toml::Value {
        fn inline(_: DefOpts, _: &[DataType]) -> Result<DataType, ExportError> {
            Ok(DataType::Any)
        }
    }

    #[derive(Type)]
    #[specta(remote = toml::value::Date, crate = crate, export = false)]
    #[allow(dead_code)]
    struct Date {
        year: u16,
        month: u8,
        day: u8,
    }

    #[derive(Type)]
    #[specta(remote = toml::value::Time, crate = crate, export = false)]
    #[allow(dead_code)]
    struct Time {
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
    }

    #[derive(Type)]
    #[specta(remote = toml::value::Datetime, crate = crate, export = false)]
    #[allow(dead_code)]
    struct Datetime {
        pub date: Option<toml::value::Date>,
        pub time: Option<toml::value::Time>,
        pub offset: Option<toml::value::Offset>,
    }

    #[derive(Type)]
    #[specta(remote = toml::value::Offset, crate = crate, export = false)]
    #[allow(dead_code)]
    pub enum Offset {
        Z,
        Custom { minutes: i16 },
    }
};

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
        fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
            String::inline(opts, generics)
        }
    }

    #[allow(deprecated)]
    impl<T: TimeZone> Type for Date<T> {
        fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
            String::inline(opts, generics)
        }
    }
};

#[cfg(feature = "time")]
impl_as!(
    time::PrimitiveDateTime as String
    time::OffsetDateTime as String
    time::Date as String
    time::Time as String
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
    use uhlc::*;

    impl_as!(
        NTP64 as u64
        ID as NonZeroU128
    );

    #[derive(Type)]
    #[specta(remote = Timestamp, crate = crate, export = false)]
    #[allow(dead_code)]
    struct Timestamp {
        time: NTP64,
        id: ID,
    }
};

#[cfg(feature = "glam")]
const _: () = {
    use glam::*;

    #[derive(Type)]
    #[specta(remote = DVec2, crate = crate, export = false)]
    #[allow(dead_code)]
    struct DVec2 {
        x: f64,
        y: f64,
    }

    #[derive(Type)]
    #[specta(remote = IVec2, crate = crate, export = false)]
    #[allow(dead_code)]
    struct IVec2 {
        x: i32,
        y: i32,
    }

    #[derive(Type)]
    #[specta(remote = DMat2, crate = crate, export = false)]
    #[allow(dead_code)]
    struct DMat2 {
        pub x_axis: DVec2,
        pub y_axis: DVec2,
    }

    #[derive(Type)]
    #[specta(remote = DAffine2, crate = crate, export = false)]
    #[allow(dead_code)]
    struct DAffine2 {
        matrix2: DMat2,
        translation: DVec2,
    }
};

#[cfg(feature = "url")]
impl_as!(url::Url as String);

#[cfg(feature = "either")]
impl<L: Type, R: Type> Type for either::Either<L, R> {
    fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType, ExportError> {
        Ok(DataType::Enum(EnumType::Untagged(UntaggedEnum {
            variants: vec![
                EnumVariant::Unnamed(TupleType {
                    fields: vec![L::inline(
                        DefOpts {
                            parent_inline: opts.parent_inline,
                            type_map: opts.type_map,
                        },
                        generics,
                    )?],
                    generics: vec![],
                }),
                EnumVariant::Unnamed(TupleType {
                    fields: vec![R::inline(
                        DefOpts {
                            parent_inline: opts.parent_inline,
                            type_map: opts.type_map,
                        },
                        generics,
                    )?],
                    generics: vec![],
                }),
            ],
            generics: vec![],
        })))
    }
}
