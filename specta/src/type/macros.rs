macro_rules! _impl_passthrough {
    ($t:ty) => {
        fn inline(type_map: &mut TypeCollection, generics: Generics) -> DataType {
            <$t>::inline(type_map, generics)
        }

        fn reference(type_map: &mut TypeCollection, generics: &[DataType]) -> Option<Reference> {
            <$t>::reference(type_map, generics)
        }
    };
}

macro_rules! _impl_primitives {
    ($($i:ident)+) => {$(
        impl Type for $i {
            fn inline(_: &mut TypeCollection, _: Generics) -> DataType {
                DataType::Primitive(datatype::PrimitiveType::$i)
            }
        }
    )+};
}

macro_rules! _impl_tuple {
    ( impl $($i:ident),* ) => {
        #[allow(non_snake_case)]
        impl<$($i: Type),*> Type for ($($i,)*) {
            #[allow(unused)]
            fn inline(type_map: &mut TypeCollection, generics: Generics) -> DataType {
                let generics = match generics {
                    Generics::Definition => &[],
                    Generics::Provided(generics) => generics,
                };

                let mut _generics = generics.iter();
                $(let $i = _generics.next().map(Clone::clone).unwrap_or_else(
                    || {
                        crate::datatype::reference::reference_or_inline::<$i>(type_map, generics)
                    },
                );)*

                datatype::TupleType {
                    elements: vec![$($i),*],
                }.to_anonymous()
            }
        }
    };
    ( $i2:ident $(, $i:ident)* ) => {
        impl_tuple!(impl $($i),* );
        impl_tuple!($($i),*);
    };
    () => {};
}

macro_rules! _impl_containers {
    ($($container:ident)+) => {$(
        impl<T: Type> Type for $container<T> {
            fn inline(type_map: &mut TypeCollection, generics: Generics) -> DataType {
                let _generics = match generics {
                    Generics::Definition => &[],
                    Generics::Provided(generics) => generics,
                };

                _generics.get(0).cloned().unwrap_or_else(
                    || {
                        T::inline(type_map, generics)
                    },
                )
            }

            fn reference(type_map: &mut TypeCollection, generics: &[DataType]) -> Option<Reference> {
                None
            }
        }

        impl<T: NamedType> NamedType for $container<T> {
            fn sid() -> SpectaID {
                T::sid()
            }

            // fn named_data_type(type_map: &mut TypeCollection, generics: &[DataType]) -> NamedDataType {
            //     T::named_data_type(type_map, generics)
            // }

            fn definition_named_data_type(type_map: &mut TypeCollection) -> NamedDataType {
                T::definition_named_data_type(type_map)
            }
        }

        impl<T: Flatten> Flatten for $container<T> {}
    )+}
}

macro_rules! _impl_as {
    ($($ty:path as $tty:ident)+) => {$(
        impl Type for $ty {
            fn inline(type_map: &mut TypeCollection, generics: Generics) -> DataType {
                <$tty as Type>::inline(type_map, generics)
            }

            fn reference(type_map: &mut TypeCollection, generics: &[DataType]) -> Option<Reference> {
                <$tty as Type>::reference(type_map, generics)
            }
        }
    )+};
}

macro_rules! _impl_for_list {
    ($($unique:expr; $ty:path as $name:expr)+) => {$(
        impl<T: Type> Type for $ty {
            fn inline(type_map: &mut TypeCollection, generics: Generics) -> DataType {
                let _generics = match generics {
                    Generics::Definition => &[],
                    Generics::Provided(generics) => generics,
                };

                DataType::List(List {
                    ty: Box::new(_generics.get(0).cloned().unwrap_or_else(|| T::inline(
                        type_map,
                        generics,
                    ))),
                    length: None,
                    unique: $unique,
                })
            }

            fn reference(type_map: &mut TypeCollection, generics: &[DataType]) -> Option<Reference> {
                None
            }
        }
    )+};
}

macro_rules! _impl_for_map {
    ($ty:path as $name:expr) => {
        impl<K: Type, V: Type> Type for $ty {
            fn inline(type_map: &mut TypeCollection, generics: Generics) -> DataType {
                let _generics = match generics {
                    Generics::Definition => &[],
                    Generics::Provided(generics) => generics,
                };

                DataType::Map(crate::datatype::Map {
                    key_ty: Box::new(
                        _generics
                            .get(0)
                            .cloned()
                            .unwrap_or_else(|| K::inline(type_map, generics)),
                    ),
                    value_ty: Box::new(
                        _generics
                            .get(1)
                            .cloned()
                            .unwrap_or_else(|| V::inline(type_map, generics)),
                    ),
                })
            }

            fn reference(type_map: &mut TypeCollection, generics: &[DataType]) -> Option<Reference> {
                None
            }
        }
    };
}

pub(crate) use _impl_as as impl_as;
pub(crate) use _impl_containers as impl_containers;
pub(crate) use _impl_for_list as impl_for_list;
pub(crate) use _impl_for_map as impl_for_map;
pub(crate) use _impl_passthrough as impl_passthrough;
pub(crate) use _impl_primitives as impl_primitives;
pub(crate) use _impl_tuple as impl_tuple;
