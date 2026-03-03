use std::borrow::Cow;

use crate::{
    Type, TypeCollection,
    datatype::{self, DataType},
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

// Technically we only support 12-tuples but the `T13` is required due to how the macro works
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);

#[cfg(feature = "std")]
const _: () = {
    use std::{
        cell::{Cell, RefCell},
        collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque},
        convert::Infallible,
        ffi::{CStr, CString, OsStr, OsString},
        net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
        num::{
            NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
            NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
        },
        ops::{Range, RangeInclusive},
        path::{Path, PathBuf},
        rc::Rc,
        sync::{
            Arc, Mutex, RwLock,
            atomic::{
                AtomicBool, AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize, AtomicU8,
                AtomicU16, AtomicU32, AtomicU64, AtomicUsize,
            },
        },
        time::{Duration, SystemTime},
    };

    use crate::{
        datatype::{Enum, EnumVariant, Field, List},
        internal,
    };

    impl Type for String {
        impl_passthrough!(str);
    }

    impl_ndt_as!(
        // Non-unique sets
        // Vec is the base impl
        // VecDeque<T> as Vec<T>
        // BinaryHeap<T> as Vec<T>
        // LinkedList<T> as Vec<T>,

        // // Unique sets
        // // HashSet is the base impl
        // BTreeSet<T> as HashSet<T>,

        // // Maps
        // // HashMap is the base impl
        // BTreeMap<K, V> as HashMap<K, V>

        // // Containers
        Box<T> as T
        // Rc<T> as T
        // Arc<T> as T
        // Cell<T> as T
        // RefCell<T> as T

        // Mutex<T> as T
        // RwLock<T> as T

        // CString<> as str
        // CStr<> as String
        // OsString<> as String
        // OsStr<> as String

        // Path<> as String
        // PathBuf<> as String

        // IpAddr<> as String
        // Ipv4Addr<> as String
        // Ipv6Addr<> as String

        // SocketAddr<> as String
        // SocketAddrV4<> as String
        // SocketAddrV6<> as String

        // AtomicBool<> as bool
        // AtomicI8<> as i8
        // AtomicI16<> as i16
        // AtomicI32<> as i32
        // AtomicIsize<> as isize
        // AtomicU8<> as u8
        // AtomicU16<> as u16
        // AtomicU32<> as u32
        // AtomicUsize<> as usize
        // AtomicI64<> as i64
        // AtomicU64<> as u64

        // NonZeroU8<> as u8
        // NonZeroU16<> as u16
        // NonZeroU32<> as u32
        // NonZeroU64<> as u64
        // NonZeroUsize<> as usize
        // NonZeroI8<> as i8
        // NonZeroI16<> as i16
        // NonZeroI32<> as i32
        // NonZeroI64<> as i64
        // NonZeroIsize<> as isize
        // NonZeroU128<> as u128
        // NonZeroI128<> as i128

        // // Serde are cringe so this is how it is :(
        // RangeInclusive<T> as Range<T>
    );

    impl_ndt!(
        impl<T: Type> Type for Vec<T> {
            inline: true;
            build: |types, ndt| {
                let mut l = List::new(
                    <T as Type>::definition(types),
                );
                l.set_unique(false);
                ndt.inner = DataType::List(l);
            }
        }

        impl<T: Type> Type for HashSet<T> {
            inline: true;
            build: |types, ndt| {
                let mut l = List::new(
                    <T as Type>::definition(types),
                );
                l.set_unique(true);
                ndt.inner = DataType::List(l);
            }
        }

        impl<K: Type, V: Type> Type for HashMap<K, V> {
            inline: true;
            build: |types, ndt| {
                ndt.inner = DataType::Map(crate::datatype::Map::new(
                    K::definition(types),
                    V::definition(types),
                ));
            }
        }

        impl<> Type for Infallible {
            inline: true;
            build: |types, ndt| {
                // Serde does no support `Infallible` as it can't be constructed as a `&self` method is uncallable on it.
                ndt.inner = DataType::Enum(Enum::default());
            }
        }

        impl<T: Type, E: Type> Type for Result<T, E> {
            inline: true;
            build: |types, ndt| {
                let mut ok_variant = EnumVariant::unit();
                ok_variant.set_fields(internal::construct::fields_unnamed(
                    vec![Field::new(T::definition(types))],
                    vec![],
                ));
                let mut err_variant = EnumVariant::unit();
                err_variant.set_fields(internal::construct::fields_unnamed(
                    vec![Field::new(E::definition(types))],
                    vec![],
                ));
                ndt.inner = DataType::Enum(Enum {
                    variants: vec![("Ok".into(), ok_variant), ("Err".into(), err_variant)],
                    attributes: vec![],
                });
            }
        }

        impl<T: Type> Type for Range<T> {
            inline: true;
            build: |types, ndt| {
                let ty = T::definition(types);
                let mut s = crate::datatype::Struct::unit();
                s.set_fields(internal::construct::fields_named(
                    vec![
                        ("start".into(), Field::new(ty.clone())),
                        ("end".into(), Field::new(ty)),
                    ],
                    vec![],
                ));

                ndt.inner = DataType::Struct(s);
            }
        }

        impl<> Type for SystemTime {
            inline: true;
            build: |types, ndt| {
                let mut s = crate::datatype::Struct::unit();
                s.set_fields(internal::construct::fields_named(
                    vec![
                        (
                            "duration_since_epoch".into(),
                            Field::new(<i64 as crate::Type>::definition(types)),
                        ),
                        (
                            "duration_since_unix_epoch".into(),
                            Field::new(<u32 as crate::Type>::definition(types)),
                        ),
                    ],
                    vec![],
                ));

                ndt.inner = DataType::Struct(s);
            }
        }

        impl<> Type for Duration {
            inline: true;
            build: |types, ndt| {
                let mut s = crate::datatype::Struct::unit();
                s.set_fields(internal::construct::fields_named(
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
                ));

                ndt.inner = DataType::Struct(s);
            }
        }


    );

    // impl<'a, T: ?Sized + ToOwned + Type + 'a> Type for Cow<'a, T> {
    //     impl_passthrough!(T);
    // }
};

// #[cfg(feature = "tokio")]
// const _: () = {
//     use tokio::sync::{Mutex, RwLock};
//     impl_containers!(Mutex RwLock);
// };

impl<T: Type + 'static> Type for &T {
    impl_passthrough!(T);
}

impl<T: Type> Type for [T] {
    impl_passthrough!(Vec<T>);
}

impl<T: Type> Type for &[T] {
    impl_passthrough!(Vec<T>);
}

// impl<const N: usize, T: Type> Type for [T; N] {
//     fn definition(types: &mut TypeCollection) -> DataType {
//         let mut l = List::new(T::definition(types));
//         l.set_length(Some(N));
//         DataType::List(l)
//     }
// }

// impl<T: Type> Type for Option<T> {
//     fn definition(types: &mut TypeCollection) -> DataType {
//         DataType::Nullable(Box::new(T::definition(types)))
//     }
// }

// impl<T> Type for std::marker::PhantomData<T> {
//     fn definition(types: &mut TypeCollection) -> DataType {
//         // This API is internal. Use [NamedDataType::register] if you want a custom implementation.
//         static SENTINEL: &str = concat!(module_path!(), "::PhantomData<T>");
//         DataType::Reference(NamedDataType::init_with_sentinel(
//             vec![],
//             true,
//             types,
//             SENTINEL,
//             |types, ndt| ndt.inner = <() as Type>::definition(types),
//         ))
//     }
// }

// impl<T: Type, E: Type> Type for Result<T, E> {
//     fn definition(types: &mut TypeCollection) -> DataType {
//         // This API is internal. Use [NamedDataType::register] if you want a custom implementation.
//         static SENTINEL: &str = concat!(module_path!(), "::Result<T, E>");
//         DataType::Reference(NamedDataType::init_with_sentinel(
//             vec![
//                 (Generic("T".into()), T::definition(types)),
//                 (Generic("E".into()), E::definition(types)),
//             ],
//             true,
//             types,
//             SENTINEL,
//             |types, ndt| {
//                 let mut ok_variant = EnumVariant::unit();
//                 ok_variant.set_fields(internal::construct::fields_unnamed(
//                     vec![Field::new(T::definition(types))],
//                     vec![],
//                 ));

//                 let mut err_variant = EnumVariant::unit();
//                 err_variant.set_fields(internal::construct::fields_unnamed(
//                     vec![Field::new(E::definition(types))],
//                     vec![],
//                 ));

//                 ndt.inner = DataType::Enum(Enum {
//                     variants: vec![("Ok".into(), ok_variant), ("Err".into(), err_variant)],
//                     attributes: vec![],
//                 })
//             },
//         ))
//     }
// }
