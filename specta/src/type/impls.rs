use crate::{datatype::*, r#type::macros::*, *};

use std::borrow::Cow;

impl_primitives!(
    i8 i16 i32 i64 i128 isize
    u8 u16 u32 u64 u128 usize
    f32 f64
    bool char
    String
);

#[cfg(feature = "nightly")]
impl Type for f16 {
    fn definition(_: &mut TypeCollection) -> DataType {
        DataType::Primitive(datatype::Primitive::f16)
    }
}

impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13); // Technically we only support 12-tuples but the `T13` is required due to how the macro works

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
    impl_passthrough!(String);
}

impl Type for Box<str> {
    impl_passthrough!(String);
}

impl<'a, T: Type + 'static> Type for &'a T {
    impl_passthrough!(T);
}

impl<T: Type> Type for [T] {
    impl_passthrough!(Vec<T>);
}

impl<'a, T: ?Sized + ToOwned + Type + 'static> Type for std::borrow::Cow<'a, T> {
    impl_passthrough!(T);
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
    false; Vec<T> as "Vec"
    false; VecDeque<T> as "VecDeque"
    false; BinaryHeap<T> as "BinaryHeap"
    false; LinkedList<T> as "LinkedList"
    true; HashSet<T> as "HashSet"
    true; BTreeSet<T> as "BTreeSet"
);

impl<'a, T: Type> Type for &'a [T] {
    impl_passthrough!(Vec<T>);
}

impl<const N: usize, T: Type> Type for [T; N] {
    fn definition(types: &mut TypeCollection) -> DataType {
        DataType::List(List::new(T::definition(types), Some(N), false))
    }
}

impl<T: Type> Type for Option<T> {
    fn definition(types: &mut TypeCollection) -> DataType {
        DataType::Nullable(Box::new(T::definition(types)))
    }
}

impl<T> Type for std::marker::PhantomData<T> {
    fn definition(_: &mut TypeCollection) -> DataType {
        DataType::Literal(Literal::None)
    }
}

// Serde does no support `Infallible` as it can't be constructed as a `&self` method is uncallable on it.
const _: () = {
    impl Type for std::convert::Infallible {
        fn definition(_: &mut TypeCollection) -> DataType {
            DataType::Enum(internal::construct::r#enum(
                Some(EnumRepr::External),
                vec![],
            ))
        }
    }
};

impl<T: Type> Type for std::ops::Range<T> {
    fn definition(types: &mut TypeCollection) -> DataType {
        let ty = Some(T::definition(types));
        DataType::Struct(Struct {
            fields: Fields::Named(NamedFields {
                fields: vec![
                    (
                        "start".into(),
                        Field {
                            optional: false,
                            flatten: false,
                            deprecated: None,
                            docs: Cow::Borrowed(""),
                            inline: false,
                            ty: ty.clone(),
                        },
                    ),
                    (
                        "end".into(),
                        Field {
                            optional: false,
                            flatten: false,
                            deprecated: None,
                            docs: Cow::Borrowed(""),
                            inline: false,
                            ty,
                        },
                    ),
                ],
                tag: None,
            }),
        })
    }
}

impl<T: Type> Flatten for std::ops::Range<T> {}

impl<T: Type> Type for std::ops::RangeInclusive<T> {
    impl_passthrough!(std::ops::Range<T>); // Yeah Serde are cringe
}

impl<T: Type> Flatten for std::ops::RangeInclusive<T> {}

impl_for_map!(HashMap<K, V> as "HashMap");
impl_for_map!(BTreeMap<K, V> as "BTreeMap");
impl<K: Type, V: Type> Flatten for std::collections::HashMap<K, V> {}
impl<K: Type, V: Type> Flatten for std::collections::BTreeMap<K, V> {}

const _: () = {
    const SID: SpectaID = internal::construct::sid("SystemTime", "::type::impls:305:10");

    impl Type for std::time::SystemTime {
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

    #[automatically_derived]
    impl Flatten for std::time::SystemTime {}
};

const _: () = {
    const SID: SpectaID = internal::construct::sid("Duration", "::type::impls:401:10");

    impl Type for std::time::Duration {
        fn definition(types: &mut TypeCollection) -> DataType {
            DataType::Struct(internal::construct::r#struct(
                internal::construct::fields_named(
                    vec![
                        (
                            "secs".into(),
                            internal::construct::field::<u64>(false, false, None, "".into(), types),
                        ),
                        (
                            "nanos".into(),
                            internal::construct::field::<u32>(false, false, None, "".into(), types),
                        ),
                    ],
                    None,
                ),
            ))
        }
    }

    impl Flatten for std::time::Duration {}
};
