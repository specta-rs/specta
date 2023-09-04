macro_rules! impl_primitives {
    ($($i:ident)+) => {$(
        impl Type for $i {
            fn inline(_: DefOpts, _: &[DataType]) -> DataType {
                DataType::Primitive(datatype::PrimitiveType::$i)
            }
        }
    )+};
}

macro_rules! impl_tuple {
    ( impl $i:ident ) => {
        impl_tuple!(impl); // This does tuple struct
    }; // T = (T1)
    ( impl $($i:ident),* ) => {
        #[allow(non_snake_case)]
        impl<$($i: Type + 'static),*> Type for ($($i),*) {
            #[allow(unused)]
            fn inline(opts: DefOpts, generics: &[DataType]) -> DataType {
                let mut _generics = generics.iter();

                $(let $i = _generics.next().map(Clone::clone).unwrap_or_else(
                    || {
                        $i::reference(
                            DefOpts {
                                parent_inline: opts.parent_inline,
                                type_map: opts.type_map,
                            },
                            generics,
                        ).inner
                    },
                );)*

                datatype::TupleType {
                    fields: vec![$($i),*],
                }.to_anonymous()
            }
        }
    };
    ( $i2:ident $(, $i:ident)* ) => {
        impl_tuple!(impl $i2 $(, $i)* );
        impl_tuple!($($i),*);
    };
    () => {};
}

macro_rules! impl_containers {
    ($($container:ident)+) => {$(
        impl<T: Type> Type for $container<T> {
            fn inline(opts: DefOpts, generics: &[DataType]) -> DataType {
                generics.get(0).cloned().unwrap_or_else(
                    || {
                        T::inline(
                           opts,
                            generics,
                        )
                    },
                )
            }

            fn reference(opts: DefOpts, generics: &[DataType]) -> Reference {
                Reference {
                    inner: generics.get(0).cloned().unwrap_or_else(
                        || T::reference(opts, generics).inner,
                    ),
                    _priv: (),
                }
            }
        }

        impl<T: NamedType> NamedType for $container<T> {
	        const SID: SpectaID = T::SID;
	        const IMPL_LOCATION: ImplLocation = T::IMPL_LOCATION;

            fn named_data_type(opts: DefOpts, generics: &[DataType]) -> NamedDataType {
                T::named_data_type(opts, generics)
            }

            fn definition_named_data_type(opts: DefOpts) -> NamedDataType {
                T::definition_named_data_type(opts)
            }
        }

        impl<T: Flatten> Flatten for $container<T> {}
    )+}
}

macro_rules! impl_as {
    ($($ty:path as $tty:ident)+) => {$(
        impl Type for $ty {
            fn inline(opts: DefOpts, generics: &[DataType]) -> DataType {
                <$tty as Type>::inline(opts, generics)
            }

            fn reference(opts: DefOpts, generics: &[DataType]) -> Reference {
                <$tty as Type>::reference(opts, generics)
            }
        }
    )+};
}

macro_rules! impl_for_list {
    ($($ty:path as $name:expr)+) => {$(
        impl<T: Type> Type for $ty {
            fn inline(opts: DefOpts, generics: &[DataType]) -> DataType {
                DataType::List(Box::new(generics.get(0).cloned().unwrap_or_else(|| T::inline(
                    opts,
                    generics,
                ))))
            }

            fn reference(opts: DefOpts, generics: &[DataType]) -> Reference {
                Reference {
                    inner: DataType::List(Box::new(generics.get(0).cloned().unwrap_or_else(
                        || T::reference(opts, generics).inner,
                    ))),
                    _priv: (),
                }
            }
        }
    )+};
}

macro_rules! impl_for_map {
    ($ty:path as $name:expr) => {
        impl<K: Type, V: Type> Type for $ty {
            fn inline(opts: DefOpts, generics: &[DataType]) -> DataType {
                DataType::Map(Box::new((
                    generics.get(0).cloned().unwrap_or_else(|| {
                        K::inline(
                            DefOpts {
                                parent_inline: opts.parent_inline,
                                type_map: opts.type_map,
                            },
                            generics,
                        )
                    }),
                    generics.get(1).cloned().unwrap_or_else(|| {
                        V::inline(
                            DefOpts {
                                parent_inline: opts.parent_inline,
                                type_map: opts.type_map,
                            },
                            generics,
                        )
                    }),
                )))
            }

            fn reference(opts: DefOpts, generics: &[DataType]) -> Reference {
                Reference {
                    inner: DataType::Map(Box::new((
                        generics.get(0).cloned().unwrap_or_else(|| {
                            K::reference(
                                DefOpts {
                                    parent_inline: opts.parent_inline,
                                    type_map: opts.type_map,
                                },
                                generics,
                            )
                            .inner
                        }),
                        generics.get(1).cloned().unwrap_or_else(|| {
                            V::reference(
                                DefOpts {
                                    parent_inline: opts.parent_inline,
                                    type_map: opts.type_map,
                                },
                                generics,
                            )
                            .inner
                        }),
                    ))),
                    _priv: (),
                }
            }
        }
    };
}
