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

macro_rules! _impl_passthrough {
    ($t:ty) => {
        fn definition(types: &mut TypeCollection) -> DataType {
            <$t>::definition(types)
        }
    };
}

macro_rules! _impl_ndt_as_item {
    ( $ty:ident<$($generic:ident),*> where { $($bounds:tt)* } as $ty2:ty $(; where $($where:tt)* )? ) => {
        impl_ndt!(
            impl<$($generic),*> Type for $ty<$($generic),*> where { $($generic: Type,)* $($bounds)* } {
                inline: true;
                build: |types, ndt| {
                    ndt.inner = <$ty2 as Type>::definition(types);
                }
            }
        );
    };
    ( $ty:ident<$($generic:ident),*> as $ty2:ty $(; where $($where:tt)* )? ) => {
        impl_ndt!(
            impl<$($generic),*> Type for $ty<$($generic),*> where { $($generic: Type),* } {
                inline: true;
                build: |types, ndt| {
                    ndt.inner = <$ty2 as Type>::definition(types);
                }
            }
        );
    };
    ( $ty:ident where { $($bounds:tt)* } as $ty2:ty $(; where $($where:tt)* )? ) => {
        impl_ndt!(
            impl Type for $ty where { $($bounds)* } {
                inline: true;
                build: |types, ndt| {
                    ndt.inner = <$ty2 as Type>::definition(types);
                }
            }
        );
    };
    ( $ty:ident as $ty2:ty $(; where $($where:tt)* )? ) => {
        impl_ndt!(
            impl Type for $ty {
                inline: true;
                build: |types, ndt| {
                    ndt.inner = <$ty2 as Type>::definition(types);
                }
            }
        );
    };
}

macro_rules! _impl_ndt_as {
    ( $($ty:ident $(<$($generic:ident),*>)? $( where { $($bounds:tt)* } )? as $ty2:ty $(; where $($where:tt)* )? )* ) => {
        $(
            $crate::r#type::macros::impl_ndt_as_item!($ty $(<$($generic),*>)? $( where { $($bounds)* } )? as $ty2 $(; where $($where)* )?);
        )*
    };
}

macro_rules! _impl_ndt {
    (
        $(
            impl $(<$($generic:ident),*>)? Type for $ty:ty $( where { $($bounds:tt)* } )? {
                inline: $inline:expr;
                build: |$types:ident, $ndt:ident| $build:block
            }
        )+
    ) => {
        $(
            impl$(<$($generic),*>)? Type for $ty $(where $($bounds)*)? {
                fn definition(types: &mut TypeCollection) -> DataType {
                    // This API is internal. Use [NamedDataType::register] if you want a custom implementation.
                    static SENTINEL: &str = concat!(module_path!(), "::", stringify!($ty));
                    println!("TODO: {SENTINEL:?}"); // TODO
                    DataType::Reference(datatype::NamedDataType::init_with_sentinel(
                        vec![
                            $($(
                                (datatype::Generic::new(stringify!($generic)), <$generic as Type>::definition(types))
                            ),*)?
                        ],
                        $inline,
                        types,
                        SENTINEL,
                        |$types, $ndt| $build,
                    ))
                }
            }
        )+
    };
}

pub(crate) use _impl_ndt as impl_ndt;
pub(crate) use _impl_ndt_as as impl_ndt_as;
pub(crate) use _impl_ndt_as_item as impl_ndt_as_item;
pub(crate) use _impl_passthrough as impl_passthrough;
pub(crate) use _impl_primitives as impl_primitives;
pub(crate) use _impl_tuple as impl_tuple;
