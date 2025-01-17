macro_rules! _impl_passthrough {
    ($t:ty) => {
        fn definition(type_map: &mut TypeCollection) -> DataType {
            <$t>::definition(type_map)
        }
    };
}

macro_rules! _impl_primitives {
    ($($i:ident)+) => {$(
        impl Type for $i {
            fn definition(_: &mut TypeCollection) -> DataType {
                DataType::Primitive(datatype::PrimitiveType::$i)
            }
        }
    )+};
}

macro_rules! _impl_tuple {
    ( impl $($i:ident),* ) => {
        #[allow(non_snake_case)]
        impl<$($i: Type),*> Type for ($($i,)*) {
            fn definition(types: &mut TypeCollection) -> DataType {
                datatype::TupleType {
                    elements: vec![$(<$i as Type>::definition(types)),*],
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
            fn definition(types: &mut TypeCollection) -> DataType {
                <T as Type>::definition(types)
            }
        }

        impl<T: NamedType> NamedType for $container<T> {
            fn reference(type_map: &mut TypeCollection, generics: &[DataType]) -> Reference {
                T::reference(type_map, generics)
            }
        }

        impl<T: Flatten> Flatten for $container<T> {}
    )+}
}

macro_rules! _impl_as {
    ($($ty:path as $tty:ident)+) => {$(
        impl Type for $ty {
            fn definition(type_map: &mut TypeCollection) -> DataType {
                <$tty as Type>::definition(type_map)
            }
        }
    )+};
}

macro_rules! _impl_for_list {
    ($($unique:expr; $ty:path as $name:expr)+) => {$(
        impl<T: Type> Type for $ty {
            fn definition(types: &mut TypeCollection) -> DataType {
                DataType::List(List {
                    ty: Box::new(<T as Type>::definition(types)),
                    length: None,
                    unique: $unique,
                })
            }
        }
    )+};
}

macro_rules! _impl_for_map {
    ($ty:path as $name:expr) => {
        impl<K: Type, V: Type> Type for $ty {
            fn definition(type_map: &mut TypeCollection) -> DataType {
                DataType::Map(crate::datatype::Map {
                    key_ty: Box::new(K::definition(type_map)),
                    value_ty: Box::new(V::definition(type_map)),
                })
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
