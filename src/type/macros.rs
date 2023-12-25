macro_rules! impl_passthrough {
    ($t:ty) => {
        fn inline(type_map: &mut TypeMap, generics: &[DataType]) -> DataType {
            <$t>::inline(type_map, generics)
        }

        fn reference(type_map: &mut TypeMap, generics: &[DataType]) -> Reference {
            <$t>::reference(type_map, generics)
        }
    };
}

macro_rules! impl_primitives {
    ($($i:ident)+) => {$(
        impl Type for $i {
            fn inline(_: &mut TypeMap, _: &[DataType]) -> DataType {
                DataType::Primitive(datatype::PrimitiveType::$i)
            }
        }
    )+};
}

macro_rules! impl_tuple {
    ( impl $($i:ident),* ) => {
        #[allow(non_snake_case)]
        impl<$($i: Type),*> Type for ($($i,)*) {
            #[allow(unused)]
            fn inline(type_map: &mut TypeMap, generics: &[DataType]) -> DataType {
                let mut _generics = generics.iter();

                $(let $i = _generics.next().map(Clone::clone).unwrap_or_else(
                    || {
                        $i::reference(type_map, generics).inner
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

macro_rules! impl_containers {
    ($($container:ident)+) => {$(
        impl<T: Type> Type for $container<T> {
            fn inline(type_map: &mut TypeMap, generics: &[DataType]) -> DataType {
                generics.get(0).cloned().unwrap_or_else(
                    || {
                        T::inline(type_map, generics)
                    },
                )
            }

            fn reference(type_map: &mut TypeMap, generics: &[DataType]) -> Reference {
                Reference {
                    inner: generics.get(0).cloned().unwrap_or_else(
                        || T::reference(type_map, generics).inner,
                    ),
                }
            }
        }

        impl<T: NamedType> NamedType for $container<T> {
	        const SID: SpectaID = T::SID;
	        const IMPL_LOCATION: ImplLocation = T::IMPL_LOCATION;

            fn named_data_type(type_map: &mut TypeMap, generics: &[DataType]) -> NamedDataType {
                T::named_data_type(type_map, generics)
            }

            fn definition_named_data_type(type_map: &mut TypeMap) -> NamedDataType {
                T::definition_named_data_type(type_map)
            }
        }

        impl<T: Flatten> Flatten for $container<T> {}
    )+}
}

macro_rules! impl_as {
    ($($ty:path as $tty:ident)+) => {$(
        impl Type for $ty {
            fn inline(type_map: &mut TypeMap, generics: &[DataType]) -> DataType {
                <$tty as Type>::inline(type_map, generics)
            }

            fn reference(type_map: &mut TypeMap, generics: &[DataType]) -> Reference {
                <$tty as Type>::reference(type_map, generics)
            }
        }
    )+};
}

macro_rules! impl_for_list {
    ($($unique:expr; $ty:path as $name:expr)+) => {$(
        impl<T: Type> Type for $ty {
            fn inline(type_map: &mut TypeMap, generics: &[DataType]) -> DataType {
                DataType::List(List {
                    ty: Box::new(generics.get(0).cloned().unwrap_or_else(|| T::inline(
                        type_map,
                        generics,
                    ))),
                    length: None,
                    unique: $unique,
                })
            }

            fn reference(type_map: &mut TypeMap, generics: &[DataType]) -> Reference {
                Reference {
                    inner: DataType::List(List {
                        ty: Box::new(generics.get(0).cloned().unwrap_or_else(
                            || T::reference(type_map, generics).inner,
                        )),
                        length: None,
                        unique: $unique,
                    }),
                }
            }
        }
    )+};
}

macro_rules! impl_for_map {
    ($ty:path as $name:expr) => {
        impl<K: Type, V: Type> Type for $ty {
            fn inline(type_map: &mut TypeMap, generics: &[DataType]) -> DataType {
                DataType::Map(crate::datatype::Map {
                    key_ty: Box::new(
                        generics
                            .get(0)
                            .cloned()
                            .unwrap_or_else(|| K::inline(type_map, generics)),
                    ),
                    value_ty: Box::new(
                        generics
                            .get(1)
                            .cloned()
                            .unwrap_or_else(|| V::inline(type_map, generics)),
                    ),
                })
            }

            fn reference(type_map: &mut TypeMap, generics: &[DataType]) -> Reference {
                Reference {
                    inner: DataType::Map(crate::datatype::Map {
                        key_ty: Box::new(
                            generics
                                .get(0)
                                .cloned()
                                .unwrap_or_else(|| K::reference(type_map, generics).inner),
                        ),
                        value_ty: Box::new(
                            generics
                                .get(1)
                                .cloned()
                                .unwrap_or_else(|| V::reference(type_map, generics).inner),
                        ),
                    }),
                }
            }
        }
    };
}
