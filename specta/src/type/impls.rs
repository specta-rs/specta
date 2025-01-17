use crate::{datatype::reference::Reference, datatype::*, r#type::macros::*, *};

use std::borrow::Cow;

impl_primitives!(
    i8 i16 i32 i64 i128 isize
    u8 u16 u32 u64 u128 usize
    f32 f64
    bool char
    String
);

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
    fn inline(type_map: &mut TypeCollection, generics: Generics) -> DataType {
        DataType::List(List {
            ty: Box::new(
                // TODO: This is cursed. Fix it properly!!!
                match Vec::<T>::inline(type_map, generics) {
                    DataType::List(List { ty, .. }) => *ty,
                    _ => unreachable!(),
                },
            ),
            length: Some(N),
            unique: false,
        })
    }

    fn reference(type_map: &mut TypeCollection, generics: &[DataType]) -> Option<Reference> {
        None
    }
}

impl<T: Type> Type for Option<T> {
    fn inline(type_map: &mut TypeCollection, generics: Generics) -> DataType {
        let mut ty = None;
        if let Generics::Provided(generics) = &generics {
            ty = generics.get(0).cloned()
        }

        DataType::Nullable(Box::new(match ty {
            Some(ty) => ty,
            None => T::inline(type_map, generics),
        }))
    }

    fn reference(type_map: &mut TypeCollection, generics: &[DataType]) -> Option<Reference> {
        None
    }
}

impl<T> Type for std::marker::PhantomData<T> {
    fn inline(_: &mut TypeCollection, _: Generics) -> DataType {
        DataType::Literal(LiteralType::None)
    }
}

// Serde does no support `Infallible` as it can't be constructed as a `&self` method is uncallable on it.
const _: () = {
    const IMPL_LOCATION: ImplLocation =
        internal::construct::impl_location("specta/src/type/impls.rs:234:10");

    impl Type for std::convert::Infallible {
        fn inline(_: &mut TypeCollection, _: Generics) -> DataType {
            DataType::Enum(internal::construct::r#enum(
                "Infallible".into(),
                internal::construct::sid("Infallible".into(), "::todo:4:10"),
                EnumRepr::External,
                false,
                vec![],
                vec![],
            ))
        }
        fn reference(type_map: &mut TypeCollection, _: &[DataType]) -> Option<reference::Reference> {
            None
        }
    }

    impl NamedType for std::convert::Infallible {
        fn sid() -> SpectaID {
            internal::construct::sid("Infallible".into(), "::todo:234:10")
        }

        // fn named_data_type(type_map: &mut TypeCollection, generics: &[DataType]) -> NamedDataType {
        //     internal::construct::named_data_type(
        //         "Infallible".into(),
        //         "".into(),
        //         None,
        //         Self::sid(),
        //         IMPL_LOCATION,
        //         <Self as Type>::inline(type_map, Generics::Provided(generics)),
        //     )
        // }
        fn definition_named_data_type(type_map: &mut TypeCollection) -> NamedDataType {
            internal::construct::named_data_type(
                "Infallible".into(),
                "".into(),
                None,
                Self::sid(),
                IMPL_LOCATION,
                <Self as Type>::inline(type_map, Generics::Definition),
            )
        }
    }
};

impl<T: Type> Type for std::ops::Range<T> {
    fn inline(type_map: &mut TypeCollection, _generics: Generics) -> DataType {
        let ty = Some(T::inline(type_map, Generics::Definition));
        DataType::Struct(StructType {
            name: "Range".into(),
            sid: None,
            generics: vec![],
            fields: Fields::Named(NamedFields {
                fields: vec![
                    (
                        "start".into(),
                        Field {
                            optional: false,
                            flatten: false,
                            deprecated: None,
                            docs: Cow::Borrowed(""),
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
    const IMPL_LOCATION: ImplLocation =
        internal::construct::impl_location("specta/src/type/impls.rs:302:10");

    impl Type for std::time::SystemTime {
        fn inline(type_map: &mut TypeCollection, _: Generics) -> DataType {
            DataType::Struct(internal::construct::r#struct(
                "SystemTime".into(),
                Some(internal::construct::sid("SystemTime".into(), "::todo:3:10")),
                vec![],
                internal::construct::fields_named(
                    vec![
                        (
                            "duration_since_epoch".into(),
                            internal::construct::field(
                                false,
                                false,
                                None,
                                "".into(),
                                Some({
                                    let ty = <i64 as Type>::inline(type_map, Generics::Provided(&[]));
                                    ty
                                }),
                            ),
                        ),
                        (
                            "duration_since_unix_epoch".into(),
                            internal::construct::field(
                                false,
                                false,
                                None,
                                "".into(),
                                Some({
                                    let ty = <u32 as Type>::inline(type_map, Generics::Provided(&[]));
                                    ty
                                }),
                            ),
                        ),
                    ],
                    None,
                ),
            ))
        }

        fn reference(type_map: &mut TypeCollection, _: &[DataType]) -> Option<reference::Reference> {
            Some(
                reference::reference::<Self>(
                    type_map,
                    internal::construct::data_type_reference("SystemTime".into(), SID, vec![]),
                )
            )
        }
    }

    impl NamedType for std::time::SystemTime {
        fn sid() -> SpectaID {
            SID
        }
        // fn named_data_type(type_map: &mut TypeCollection, generics: &[DataType]) -> NamedDataType {
        //     internal::construct::named_data_type(
        //         "SystemTime".into(),
        //         "".into(),
        //         None,
        //         Self::sid(),
        //         IMPL_LOCATION,
        //         <Self as Type>::inline(type_map, Generics::Provided(generics)),
        //     )
        // }
        fn definition_named_data_type(type_map: &mut TypeCollection) -> NamedDataType {
            internal::construct::named_data_type(
                "SystemTime".into(),
                "".into(),
                None,
                Self::sid(),
                IMPL_LOCATION,
                <Self as Type>::inline(type_map, Generics::Definition),
            )
        }
    }
    #[automatically_derived]
    impl Flatten for std::time::SystemTime {}
};

const _: () = {
    const SID: SpectaID = internal::construct::sid("Duration", "::type::impls:401:10");
    const IMPL_LOCATION: ImplLocation =
        internal::construct::impl_location("specta/src/type/impls.rs:401:10");

    impl Type for std::time::Duration {
        fn inline(type_map: &mut TypeCollection, _: Generics) -> DataType {
            DataType::Struct(internal::construct::r#struct(
                "Duration".into(),
                Some(SID),
                vec![],
                internal::construct::fields_named(
                    vec![
                        (
                            "secs".into(),
                            internal::construct::field(
                                false,
                                false,
                                None,
                                "".into(),
                                Some({
                                    let ty = <u64 as Type>::inline(type_map, Generics::Definition);
                                    ty
                                }),
                            ),
                        ),
                        (
                            "nanos".into(),
                            internal::construct::field(
                                false,
                                false,
                                None,
                                "".into(),
                                Some({
                                    let ty = <u32 as Type>::inline(type_map, Generics::Definition);
                                    ty
                                }),
                            ),
                        ),
                    ],
                    None,
                ),
            ))
        }
        fn reference(type_map: &mut TypeCollection, _: &[DataType]) -> Option<reference::Reference> {
            Some(
                reference::reference::<Self>(
                    type_map,
                    internal::construct::data_type_reference("Duration".into(), Self::sid(), vec![]),
                )
            )
        }
    }

    impl NamedType for std::time::Duration {
        fn sid() -> SpectaID {
            SID
        }
        // fn named_data_type(type_map: &mut TypeCollection, generics: &[DataType]) -> NamedDataType {
        //     internal::construct::named_data_type(
        //         "Duration".into(),
        //         "".into(),
        //         None,
        //         Self::sid(),
        //         IMPL_LOCATION,
        //         <Self as Type>::inline(type_map, Generics::Provided(generics)),
        //     )
        // }
        fn definition_named_data_type(type_map: &mut TypeCollection) -> NamedDataType {
            internal::construct::named_data_type(
                "Duration".into(),
                "".into(),
                None,
                Self::sid(),
                IMPL_LOCATION,
                <Self as Type>::inline(type_map, Generics::Definition),
            )
        }
    }

    impl Flatten for std::time::Duration {}
};
