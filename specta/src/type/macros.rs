macro_rules! _impl_passthrough {
    ($t:ty) => {
        fn definition(types: &mut TypeCollection) -> DataType {
            <$t>::definition(types)
        }
    };
}

macro_rules! _impl_primitives {
    ($($i:ident)+) => {$(
        impl Type for $i {
            fn definition(_: &mut TypeCollection) -> DataType {
                DataType::Primitive(datatype::Primitive::$i)
            }
        }
    )+};
}

macro_rules! _impl_tuple {
    ( impl $($i:ident),* ) => {
        #[allow(non_snake_case)]
        impl<$($i: Type),*> Type for ($($i,)*) {
            fn definition(_types: &mut TypeCollection) -> DataType {
                DataType::Tuple(datatype::Tuple {
                    elements: vec![$(<$i as Type>::definition(_types)),*],
                })
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

        impl<T: Flatten> Flatten for $container<T> {}
    )+}
}

macro_rules! _impl_as {
    ($($ty:path as $tty:ident)+) => {$(
        impl Type for $ty {
            fn definition(types: &mut TypeCollection) -> DataType {
                <$tty as Type>::definition(types)
            }
        }

        // TODO: ????
        // impl NamedType for $ty {
        //     const ID: SpectaID = <$tty as NamedType>::ID;
        // }
    )+};
}

macro_rules! _impl_for_list {
    ($($unique:expr; $ty:path as $name:expr)+) => {$(
        impl<T: Type> Type for $ty {
            fn definition(types: &mut TypeCollection) -> DataType {
                let mut l = List::new(
                    <T as Type>::definition(types),
                );
                l.set_unique($unique);
                DataType::List(l)
            }
        }
    )+};
}

macro_rules! _impl_for_map {
    ($ty:path as $name:expr) => {
        impl<K: Type, V: Type> Type for $ty {
            fn definition(types: &mut TypeCollection) -> DataType {
                DataType::Map(crate::datatype::Map::new(
                    K::definition(types),
                    V::definition(types),
                ))
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
