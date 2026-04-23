use std::marker::PhantomData;

use crate::{
    Type, Types,
    datatype::{
        self, DataType, Enum, Field, List, NamedDataType, NamedReference, Reference, Variant,
    },
    r#type::macros::*,
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
        l.unique = true;
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
    impl_ndt!(
        std::string::String as str = inline;

        // Non-unique sets
        std::vec::Vec<T> as [T] = inline_passthrough;
        std::collections::VecDeque<T> as [T] = inline_passthrough;
        std::collections::BinaryHeap<T> as [T] = inline_passthrough;
        std::collections::LinkedList<T> as [T] = inline_passthrough;

        // Unique sets
        std::collections::HashSet<T> as PrimitiveSet<T> = inline_passthrough;
        std::collections::BTreeSet<T> as PrimitiveSet<T> = inline_passthrough;

        // Maps
        std::collections::HashMap<K, V> as PrimitiveMap<K, V> = inline_passthrough;
        std::collections::BTreeMap<K, V> as PrimitiveMap<K, V> = inline_passthrough;

        // Containers
        std::boxed::Box<T> where { T: ?Sized } as T = inline_passthrough;
        std::rc::Rc<T> where { T: ?Sized } as T = inline_passthrough;
        std::sync::Arc<T> where { T: ?Sized } as T = inline_passthrough;
        std::cell::Cell<T> where { T: ?Sized } as T = inline_passthrough;
        std::cell::RefCell<T> where { T: ?Sized } as T = inline_passthrough;

        std::sync::Mutex<T> where { T: ?Sized } as T = inline_passthrough;
        std::sync::RwLock<T> where { T: ?Sized } as T = inline_passthrough;

        std::ffi::CString as str = inline;
        std::ffi::CStr as str = inline;
        std::ffi::OsString as str = inline;
        std::ffi::OsStr as str = inline;

        std::path::Path as str = inline;
        std::path::PathBuf as str = inline;

        std::net::IpAddr as str = inline;
        std::net::Ipv4Addr as str = inline;
        std::net::Ipv6Addr as str = inline;

        std::net::SocketAddr as str = inline;
        std::net::SocketAddrV4 as str = inline;
        std::net::SocketAddrV6 as str = inline;

        std::sync::atomic::AtomicBool as bool = inline;
        std::sync::atomic::AtomicI8 as i8 = inline;
        std::sync::atomic::AtomicI16 as i16 = inline;
        std::sync::atomic::AtomicI32 as i32 = inline;
        std::sync::atomic::AtomicIsize as isize = inline;
        std::sync::atomic::AtomicU8 as u8 = inline;
        std::sync::atomic::AtomicU16 as u16 = inline;
        std::sync::atomic::AtomicU32 as u32 = inline;
        std::sync::atomic::AtomicUsize as usize = inline;
        std::sync::atomic::AtomicI64 as i64 = inline;
        std::sync::atomic::AtomicU64 as u64 = inline;

        std::num::NonZeroU8 as u8 = inline;
        std::num::NonZeroU16 as u16 = inline;
        std::num::NonZeroU32 as u32 = inline;
        std::num::NonZeroU64 as u64 = inline;
        std::num::NonZeroUsize as usize = inline;
        std::num::NonZeroI8 as i8 = inline;
        std::num::NonZeroI16 as i16 = inline;
        std::num::NonZeroI32 as i32 = inline;
        std::num::NonZeroI64 as i64 = inline;
        std::num::NonZeroIsize as isize = inline;
        std::num::NonZeroU128 as u128 = inline;
        std::num::NonZeroI128 as i128 = inline;

        // Serde are cringe so this is how it is :(
        std::ops::Range<T> as BaseRange<T> = named;
        std::ops::RangeInclusive<T> as BaseRange<T> = named;

        std::time::SystemTime as BaseSystemTime = named;
        std::time::Duration as BaseDuration = named;

        std::convert::Infallible as BaseInfallible = inline;
        std::marker::PhantomData<T> as () = inline;
        std::borrow::Cow<'a, T> where { T: ?Sized + ToOwned + 'a } as T = named;

        std::result::Result<T, E> as BaseResult<T, E> = named;
    );

    struct BaseInfallible;
    impl Type for BaseInfallible {
        fn definition(_: &mut Types) -> DataType {
            // Serde does no support `Infallible` as it can't be constructed as a `&self` method is uncallable on it.
            DataType::Enum(Enum::default())
        }
    }

    struct BaseSystemTime;
    impl Type for BaseSystemTime {
        fn definition(types: &mut Types) -> DataType {
            datatype::Struct::named()
                .field(
                    "duration_since_epoch",
                    Field::new(<i64 as crate::Type>::definition(types)),
                )
                .field(
                    "duration_since_unix_epoch",
                    Field::new(<u32 as crate::Type>::definition(types)),
                )
                .build()
        }
    }

    struct BaseDuration;
    impl Type for BaseDuration {
        fn definition(types: &mut Types) -> DataType {
            datatype::Struct::named()
                .field("secs", Field::new(<u64 as crate::Type>::definition(types)))
                .field("nanos", Field::new(<u32 as crate::Type>::definition(types)))
                .build()
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

    struct BaseResult<T, E>(PhantomData<T>, PhantomData<E>);
    impl<T: Type, E: Type> Type for BaseResult<T, E> {
        fn definition(types: &mut Types) -> DataType {
            datatype::Struct::named()
                .field("ok", Field::new(<T as Type>::definition(types)))
                .field("err", Field::new(<E as Type>::definition(types)))
                .build()
        }
    }
};

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
impl_ndt!(
    tokio::sync::Mutex<T> where { T: ?Sized } as T = inline_passthrough;
    tokio::sync::RwLock<T> where { T: ?Sized } as T = inline_passthrough;
);

impl<T: Type + ?Sized> Type for &T {
    fn definition(types: &mut Types) -> DataType {
        T::definition(types)
    }
}

impl<T: Type> Type for [T] {
    fn definition(types: &mut Types) -> DataType {
        let mut l = List::new(<T as Type>::definition(types));
        l.unique = false;
        DataType::List(l)
    }
}

impl<const N: usize, T: Type> Type for [T; N] {
    fn definition(types: &mut Types) -> DataType {
        let mut l = List::new(T::definition(types));

        // Refer to the documentation for `Types::has_const_params` to understand this.
        // If you wanna force this use `specta_utils::FixedArray<N, T>` instead.
        if !types.has_const_params {
            l.length = Some(N);
        }

        DataType::List(l)
    }
}

impl<T: Type> Type for Option<T> {
    fn definition(types: &mut Types) -> DataType {
        DataType::Nullable(Box::new(T::definition(types)))
    }
}
