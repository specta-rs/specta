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

macro_rules! _impl_ndt_as {
    ( $($head:ident :: $( $tail:ident )::+ $(<$($generic:ident),*>)? $( where { $($bounds:tt)* } )? as $ty2:ty )* ) => {
        impl_ndt!(
            $(
                impl$(<$($generic),*>)? Type for $head::$( $tail )::+ $(<$($generic),*>)? where {
                    // `(): Sized` is meaningless and is used to add a base-condition to avoid branching in the macro.
                    (): Sized $(, $($generic: Type),*)? $(, $($bounds)*)?
                } {
                    inline: true;
                    build: |types, ndt| {
                        ndt.inner = <$ty2 as Type>::definition(types);
                    }
                }
            )*
        );
    };
    ( $($tt:tt)+ ) => {
        compile_error!("impl_ndt_as! requires a fully-qualified path in `impl Type for ...` (for example: std::time::Duration)");
    };
}

macro_rules! _impl_ndt {
    (
        $(
            impl $(<$($generic:ident),*>)? Type for $type_path:path $( where { $($bounds:tt)* } )? {
                inline: $inline:expr;
                build: |$types:ident, $ndt:ident| $build:block
            }
        )+
    ) => {
        $(
            impl$(<$($generic),*>)? Type for $type_path $(where $($bounds)*)? {
                fn definition(types: &mut TypeCollection) -> DataType {
                    // This API is internal. Use [NamedDataType::register] if you want a custom implementation.
                    static SENTINEL: &str = stringify!($type_path);
                    DataType::Reference(datatype::NamedDataType::init_with_sentinel(
                        vec![
                            $($(
                                (datatype::Generic::new(stringify!($generic)), <$generic as Type>::definition(types))
                            ),*)?
                        ],
                        $inline,
                        types,
                        SENTINEL,
                        |$types, $ndt| {
                            let type_path = stringify!($type_path)
                                .chars()
                                .filter(|c| !c.is_whitespace())
                                .collect::<String>();
                            let type_path = type_path
                                .split_once('<')
                                .map(|(path, _)| path)
                                .unwrap_or(type_path.as_str());
                            let Some((module_path, _)) = type_path.rsplit_once("::") else {
                                panic!("failed to parse module path");
                            };
                            $ndt.set_module_path(::std::borrow::Cow::Owned(module_path.to_owned()));

                            $build
                        },
                    ))
                }
            }
        )+
    };
}

pub(crate) use _impl_ndt as impl_ndt;
pub(crate) use _impl_ndt_as as impl_ndt_as;
pub(crate) use _impl_passthrough as impl_passthrough;
pub(crate) use _impl_primitives as impl_primitives;
pub(crate) use _impl_tuple as impl_tuple;
