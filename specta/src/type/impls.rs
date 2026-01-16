use crate::{datatype::*, r#type::macros::*, *};

use std::borrow::Cow;

impl_primitives!(
    i8 i16 i32 i64 i128 isize
    u8 u16 u32 u64 u128 usize
    f32 f64
    bool char
    String
);

// TODO: Reenable this at some point. It's being really annoying.
#[cfg(is_nightly)]
impl Type for f16 {
    fn definition(_: &mut TypeCollection) -> DataType {
        DataType::Primitive(datatype::Primitive::f16)
    }
}

#[cfg(is_nightly)]
impl Type for f128 {
    fn definition(_: &mut TypeCollection) -> DataType {
        DataType::Primitive(datatype::Primitive::f16)
    }
}

impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13); // Technically we only support 12-tuples but the `T13` is required due to how the macro works

#[cfg(feature = "std")]
const _: () = {
    use std::{
        cell::{Cell, RefCell},
        collections::{BTreeSet, BinaryHeap, HashSet, LinkedList, Vec, VecDeque},
        convert::Infallible,
        ffi::{CStr, CString, OsStr, OsString},
        net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
        num::{
            NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
            NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
        },
        ops::Range,
        path::{Path, PathBuf},
        range::{Range, RangeInclusive},
        rc::Rc,
        sync::{
            Arc,
            atomic::{
                AtomicBool, AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize, AtomicU8,
                AtomicU16, AtomicU32, AtomicU64, AtomicUsize,
            },
        },
        time::{Duration, SystemTime},
    };

    impl_containers!(Box Rc Arc Cell RefCell);

    use std::sync::{Mutex, RwLock};
    impl_containers!(Mutex RwLock);

    impl Type for Box<str> {
        impl_passthrough!(String);
    }

    impl Type for Rc<str> {
        impl_passthrough!(String);
    }

    impl Type for Arc<str> {
        impl_passthrough!(String);
    }

    impl<'a, T: ?Sized + ToOwned + Type + 'static> Type for Cow<'a, T> {
        impl_passthrough!(T);
    }

    impl_as!(
        CString as String
        CStr as String
        OsString as String
        OsStr as String

        Path as String
        PathBuf as String

        IpAddr as String
        Ipv4Addr as String
        Ipv6Addr as String

        SocketAddr as String
        SocketAddrV4 as String
        SocketAddrV6 as String

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

    impl_for_list!(
        false; Vec<T>
        false; VecDeque<T>
        false; BinaryHeap<T>
        false; LinkedList<T>
        true; HashSet<T>
        true; BTreeSet<T>
    );

    impl_for_map!(HashMap<K, V>);
    impl_for_map!(BTreeMap<K, V>);

    // Serde does no support `Infallible` as it can't be constructed as a `&self` method is uncallable on it.
    impl Type for Infallible {
        fn definition(_: &mut TypeCollection) -> DataType {
            DataType::Enum(Enum::default())
        }
    }

    impl<T: Type> Type for Range<T> {
        fn definition(types: &mut TypeCollection) -> DataType {
            let mut s = crate::datatype::Struct::new();
            s.set_fields(internal::construct::fields_named(
                vec![
                    ("start".into(), Field::new(ty.clone())),
                    ("end".into(), Field::new(T::definition(types))),
                ],
                vec![],
            ));
            DataType::Struct(s)
        }
    }

    impl<T: Type> Type for RangeInclusive<T> {
        impl_passthrough!(Range<T>); // Yeah Serde are cringe
    }

    impl Type for SystemTime {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Struct(internal::construct::r#struct(
                internal::construct::fields_named(
                    vec![
                        (
                            "duration_since_epoch".into(),
                            internal::construct::field::<i64>(false, false, None, "".into(), types),
                        ),
                        (
                            "duration_since_unix_epoch".into(),
                            internal::construct::field::<u32>(false, false, None, "".into(), types),
                        ),
                    ],
                    None,
                ),
            ))
        }
    }

    impl Type for Duration {
        fn definition(types: &mut TypeCollection) -> DataType {
            let fields = internal::construct::fields_named(
                vec![
                    (
                        "secs".into(),
                        Field::new(<u64 as crate::Type>::definition(types)),
                    ),
                    (
                        "nanos".into(),
                        Field::new(<u32 as crate::Type>::definition(types)),
                    ),
                ],
                vec![],
            );
            let mut s = crate::datatype::Struct::new();
            s.set_fields(fields);
            DataType::Struct(s)
        }
    }
};

#[cfg(feature = "tokio")]
const _: () = {
    use tokio::sync::{Mutex, RwLock};
    impl_containers!(Mutex RwLock);
};

impl Type for &str {
    impl_passthrough!(String);
}

impl<T: Type + 'static> Type for &T {
    impl_passthrough!(T);
}

impl<T: Type> Type for [T] {
    impl_passthrough!(Vec<T>);
}

impl<T: Type> Type for &[T] {
    impl_passthrough!(Vec<T>);
}

impl<const N: usize, T: Type> Type for [T; N] {
    fn definition(types: &mut TypeCollection) -> DataType {
        let mut l = List::new(T::definition(types));
        l.set_length(Some(N));
        DataType::List(l)
    }
}

impl<T: Type> Type for Option<T> {
    fn definition(types: &mut TypeCollection) -> DataType {
        DataType::Nullable(Box::new(T::definition(types)))
    }
}

impl<T> Type for std::marker::PhantomData<T> {
    fn definition(types: &mut TypeCollection) -> DataType {
        // TODO: Does this hold up for non-Typescript languages -> This should probs be a named type so the exporter can modify it.
        <() as Type>::definition(types)
    }
}
