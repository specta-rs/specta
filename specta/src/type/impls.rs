use std::marker::PhantomData;

use crate::{
    Type, Types,
    datatype::{self, DataType, Enum, Field, List, Variant},
    r#type::{generics, macros::*},
};

impl_primitives!(
    i8 i16 i32 i64 i128 isize
    u8 u16 u32 u64 u128 usize
    f32 f64
    bool char
    str
);

#[cfg(is_nightly)]
#[cfg_attr(docsrs, doc(cfg(is_nightly)))]
impl Type for f16 {
    fn definition(_: &mut Types) -> DataType {
        DataType::Primitive(datatype::Primitive::f16)
    }
}

#[cfg(is_nightly)]
#[cfg_attr(docsrs, doc(cfg(is_nightly)))]
impl Type for f128 {
    fn definition(_: &mut Types) -> DataType {
        DataType::Primitive(datatype::Primitive::f128)
    }
}

// Technically we only support 12-tuples but the `T13` is required due to how the macro works
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);

pub(crate) struct PrimitiveSet<T>(PhantomData<T>);
impl<T: Type> Type for PrimitiveSet<T> {
    fn definition(types: &mut Types) -> DataType {
        let mut l = List::new(<T as Type>::definition(types));
        l.set_unique(true);
        DataType::List(l)
    }
}

pub(crate) struct PrimitiveMap<K, V>(PhantomData<K>, PhantomData<V>);
impl<K: Type, V: Type> Type for PrimitiveMap<K, V> {
    fn definition(types: &mut Types) -> DataType {
        DataType::Map(crate::datatype::Map::new(
            K::definition(types),
            V::definition(types),
        ))
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
const _: () = {
    impl_ndt_as!(
        std::string::String as str

        // Non-unique sets
        std::vec::Vec<T> as [generics::T]
        std::collections::VecDeque<T> as [generics::T]
        std::collections::BinaryHeap<T> as [generics::T]
        std::collections::LinkedList<T> as [generics::T]

        // Unique sets
        std::collections::HashSet<T> as PrimitiveSet<generics::T>
        std::collections::BTreeSet<T> as PrimitiveSet<generics::T>

        // Maps
        std::collections::HashMap<K, V> as PrimitiveMap<generics::K, generics::V>
        std::collections::BTreeMap<K, V> as PrimitiveMap<generics::K, generics::V>

        // Containers
        std::boxed::Box<T> where { T: ?Sized } as generics::T
        std::rc::Rc<T> where { T: ?Sized } as generics::T
        std::sync::Arc<T> where { T: ?Sized } as generics::T
        std::cell::Cell<T> where { T: ?Sized } as generics::T
        std::cell::RefCell<T> where { T: ?Sized } as generics::T

        std::sync::Mutex<T> where { T: ?Sized } as generics::T
        std::sync::RwLock<T> where { T: ?Sized } as generics::T

        std::ffi::CString as str
        std::ffi::CStr as str
        std::ffi::OsString as str
        std::ffi::OsStr as str

        std::path::Path as str
        std::path::PathBuf as str

        std::net::IpAddr as str
        std::net::Ipv4Addr as str
        std::net::Ipv6Addr as str

        std::net::SocketAddr as str
        std::net::SocketAddrV4 as str
        std::net::SocketAddrV6 as str

        std::sync::atomic::AtomicBool as bool
        std::sync::atomic::AtomicI8 as i8
        std::sync::atomic::AtomicI16 as i16
        std::sync::atomic::AtomicI32 as i32
        std::sync::atomic::AtomicIsize as isize
        std::sync::atomic::AtomicU8 as u8
        std::sync::atomic::AtomicU16 as u16
        std::sync::atomic::AtomicU32 as u32
        std::sync::atomic::AtomicUsize as usize
        std::sync::atomic::AtomicI64 as i64
        std::sync::atomic::AtomicU64 as u64

        std::num::NonZeroU8 as u8
        std::num::NonZeroU16 as u16
        std::num::NonZeroU32 as u32
        std::num::NonZeroU64 as u64
        std::num::NonZeroUsize as usize
        std::num::NonZeroI8 as i8
        std::num::NonZeroI16 as i16
        std::num::NonZeroI32 as i32
        std::num::NonZeroI64 as i64
        std::num::NonZeroIsize as isize
        std::num::NonZeroU128 as u128
        std::num::NonZeroI128 as i128

        // Serde are cringe so this is how it is :(
        std::ops::Range<T> as BaseRange<generics::T>
        std::ops::RangeInclusive<T> as BaseRange<generics::T>
    );

    impl_ndt!(
        impl Type for std::convert::Infallible {
            inline: true;
            build: |_types, ndt| {
                // Serde does no support `Infallible` as it can't be constructed as a `&self` method is uncallable on it.
                ndt.inner = DataType::Enum(Enum::default());
            }
        }

        impl Type for std::time::SystemTime {
            inline: true;
            build: |types, ndt| {
                ndt.inner = datatype::Struct::named()
                    .field(
                        "duration_since_epoch",
                        Field::new(<i64 as crate::Type>::definition(types)),
                    )
                    .field(
                        "duration_since_unix_epoch",
                        Field::new(<u32 as crate::Type>::definition(types)),
                    )
                    .build();
            }
        }

        impl Type for std::time::Duration {
            inline: true;
            build: |types, ndt| {
                ndt.inner = datatype::Struct::named()
                    .field("secs", Field::new(<u64 as crate::Type>::definition(types)))
                    .field("nanos", Field::new(<u32 as crate::Type>::definition(types)))
                    .build();
            }
        }
    );

    impl<'a, T: ?Sized + ToOwned + Type + 'a> Type for std::borrow::Cow<'a, T> {
        fn definition(types: &mut Types) -> DataType {
            use std::borrow::Cow;

            use crate::datatype::GenericReference;

            // This API is internal. Use [NamedDataType::register] if you want a custom implementation.
            static SENTINEL: &str = "std::borrow::Cow<'a, T>";
            static GENERICS: &[(GenericReference, Cow<'static, str>)] = &[(
                datatype::GenericReference::new::<generics::T>(),
                std::borrow::Cow::Borrowed("T"),
            )];

            DataType::Reference(datatype::NamedDataType::init_with_sentinel(
                GENERICS,
                vec![(
                    datatype::GenericReference::new::<generics::T>(),
                    <T as Type>::definition(types),
                )],
                true,
                types,
                SENTINEL,
                |_types, ndt| {
                    *ndt.name_mut() = std::borrow::Cow::Borrowed("Cow");
                    *ndt.module_path_mut() = std::borrow::Cow::Borrowed("std::borrow");
                    ndt.inner = datatype::GenericReference::new::<generics::T>().into();
                },
            ))
        }
    }

    struct BaseRange<T>(PhantomData<T>);
    impl<T: Type> Type for BaseRange<T> {
        fn definition(types: &mut Types) -> DataType {
            let ty = T::definition(types);
            datatype::Struct::named()
                .field("start", Field::new(ty.clone()))
                .field("end", Field::new(ty))
                .build()
        }
    }
};

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
impl_ndt_as!(
    tokio::sync::Mutex<T> where { T: ?Sized } as generics::T
    tokio::sync::RwLock<T> where { T: ?Sized } as generics::T
);

impl<T: Type + ?Sized> Type for &T {
    impl_passthrough!(T);
}

impl<T: Type> Type for [T] {
    fn definition(types: &mut Types) -> DataType {
        let mut l = List::new(<T as Type>::definition(types));
        l.set_unique(false);
        DataType::List(l)
    }
}

impl<const N: usize, T: Type> Type for [T; N] {
    fn definition(types: &mut Types) -> DataType {
        let mut l = List::new(T::definition(types));
        l.set_length(Some(N));
        DataType::List(l)
    }
}

impl<T: Type> Type for Option<T> {
    fn definition(types: &mut Types) -> DataType {
        DataType::Nullable(Box::new(T::definition(types)))
    }
}

impl_ndt_as!(
    std::marker::PhantomData<T> as ()
);

impl_ndt!(
    impl<T, E> Type for std::result::Result<T, E> where { T: Type, E: Type } {
        inline: true;
        build: |types, ndt| {
            let ok_variant = Variant::unnamed()
                .field(Field::new(datatype::GenericReference::new::<generics::T>().into()))
                .build();
            let err_variant = Variant::unnamed()
                .field(Field::new(datatype::GenericReference::new::<generics::E>().into()))
                .build();
            ndt.inner = DataType::Enum(Enum {
                variants: vec![("Ok".into(), ok_variant), ("Err".into(), err_variant)],
                attributes: datatype::Attributes::default(),
            });
        }
    }
);
