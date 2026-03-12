macro_rules! _impl_primitives {
    ($($i:ident)+) => {$(
        impl Type for $i {
            fn definition(_: &mut Types) -> DataType {
                DataType::Primitive(datatype::Primitive::$i)
            }
        }
    )+};
}

macro_rules! _impl_tuple {
    ( impl $($i:ident),* ) => {
        #[allow(non_snake_case)]
        impl<$($i: Type),*> Type for ($($i,)*) {
            fn definition(_types: &mut Types) -> DataType {
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
        fn definition(types: &mut Types) -> DataType {
            <$t>::definition(types)
        }
    };
}

macro_rules! _impl_ndt_as {
    ( $($head:ident :: $( $tail:ident )::+ < $generic:ident, const $const_generic:ident: usize > $( where { $($bounds:tt)* } )? as $ty2:ty )* ) => {
        $(
            impl<$generic: Type, const $const_generic: usize> Type
                for $head::$( $tail )::+<$generic, $const_generic>
            $(where $($bounds)*)?
            {
                fn definition(types: &mut Types) -> DataType {
                    // This API is internal. Use [NamedDataType::register] if you want a custom implementation.
                    static SENTINEL: &str = stringify!($head::$( $tail )::+<$generic, $const_generic>);
                    static GENERICS: &[(datatype::GenericReference, ::std::borrow::Cow<'static, str>)] = &[
                        (
                            datatype::GenericReference::new::<generics::$generic>(),
                            ::std::borrow::Cow::Borrowed(stringify!($generic)),
                        ),
                    ];

                    DataType::Reference(datatype::NamedDataType::init_with_sentinel(
                        GENERICS,
                        vec![(
                            datatype::GenericReference::new::<generics::$generic>(),
                            <$generic as Type>::definition(types),
                        )],
                        true,
                        types,
                        SENTINEL,
                        |types, ndt| {
                            let _ = &types;

                            let (type_name, module_path) = {
                                let s = stringify!($head::$( $tail )::+<$generic, $const_generic>);
                                let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
                                let s = cleaned.split('<').next().unwrap();
                                if let Some((path, name)) = s.rsplit_once("::") {
                                    (name.to_string(), path.to_string())
                                } else {
                                    (s.to_string(), String::new())
                                }
                            };

                            ndt.set_name(::std::borrow::Cow::Owned(type_name));
                            ndt.set_module_path(::std::borrow::Cow::Owned(module_path));
                            ndt.inner = <$ty2 as Type>::definition(types);
                        },
                    ))
                }
            }
        )*
    };
    ( $($head:ident :: $( $tail:ident )::+ < const $const_generic:ident: usize > $( where { $($bounds:tt)* } )? as $ty2:ty )* ) => {
        $(
            impl<const $const_generic: usize> Type for $head::$( $tail )::+<$const_generic>
            $(where $($bounds)*)?
            {
                fn definition(types: &mut Types) -> DataType {
                    // This API is internal. Use [NamedDataType::register] if you want a custom implementation.
                    static SENTINEL: &str = stringify!($head::$( $tail )::+<$const_generic>);
                    static GENERICS: &[(datatype::GenericReference, ::std::borrow::Cow<'static, str>)] = &[];

                    DataType::Reference(datatype::NamedDataType::init_with_sentinel(
                        GENERICS,
                        vec![],
                        true,
                        types,
                        SENTINEL,
                        |types, ndt| {
                            let _ = &types;

                            let (type_name, module_path) = {
                                let s = stringify!($head::$( $tail )::+<$const_generic>);
                                let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
                                let s = cleaned.split('<').next().unwrap();
                                if let Some((path, name)) = s.rsplit_once("::") {
                                    (name.to_string(), path.to_string())
                                } else {
                                    (s.to_string(), String::new())
                                }
                            };

                            ndt.set_name(::std::borrow::Cow::Owned(type_name));
                            ndt.set_module_path(::std::borrow::Cow::Owned(module_path));
                            ndt.inner = <$ty2 as Type>::definition(types);
                        },
                    ))
                }
            }
        )*
    };
    ( $($head:ident :: $( $tail:ident )::+ < [ $generic:ident ; $const_generic:ident ] > $( where { $($bounds:tt)* } )? as $ty2:ty )* ) => {
        $(
            impl<$generic: Type, const $const_generic: usize> Type
                for $head::$( $tail )::+<[$generic; $const_generic]>
            $(where $($bounds)*)?
            {
                fn definition(types: &mut Types) -> DataType {
                    // This API is internal. Use [NamedDataType::register] if you want a custom implementation.
                    static SENTINEL: &str = stringify!($head::$( $tail )::+<[$generic; $const_generic]>);
                    static GENERICS: &[(datatype::GenericReference, ::std::borrow::Cow<'static, str>)] = &[
                        (
                            datatype::GenericReference::new::<generics::$generic>(),
                            ::std::borrow::Cow::Borrowed(stringify!($generic)),
                        ),
                    ];

                    DataType::Reference(datatype::NamedDataType::init_with_sentinel(
                        GENERICS,
                        vec![(
                            datatype::GenericReference::new::<generics::$generic>(),
                            <$generic as Type>::definition(types),
                        )],
                        true,
                        types,
                        SENTINEL,
                        |types, ndt| {
                            let _ = &types;

                            let (type_name, module_path) = {
                                let s = stringify!($head::$( $tail )::+<[$generic; $const_generic]>);
                                let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
                                let s = cleaned.split('<').next().unwrap();
                                if let Some((path, name)) = s.rsplit_once("::") {
                                    (name.to_string(), path.to_string())
                                } else {
                                    (s.to_string(), String::new())
                                }
                            };

                            ndt.set_name(::std::borrow::Cow::Owned(type_name));
                            ndt.set_module_path(::std::borrow::Cow::Owned(module_path));
                            ndt.inner = <$ty2 as Type>::definition(types);
                        },
                    ))
                }
            }
        )*
    };
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
                fn definition(types: &mut Types) -> DataType {
                    // This API is internal. Use [NamedDataType::register] if you want a custom implementation.
                    static SENTINEL: &str = stringify!($type_path);
                    static GENERICS: &[(datatype::GenericReference, ::std::borrow::Cow<'static, str>)] = &[
                        $($(
                            (
                                datatype::GenericReference::new::<generics::$generic>(),
                                ::std::borrow::Cow::Borrowed(stringify!($generic)),
                            )
                        ),*)?
                    ];
                    DataType::Reference(datatype::NamedDataType::init_with_sentinel(
                        GENERICS,
                        vec![
                            $($(
                                (
                                    datatype::GenericReference::new::<generics::$generic>(),
                                    <$generic as Type>::definition(types),
                                )
                            ),*)?
                        ],
                        $inline,
                        types,
                        SENTINEL,
                        |$types, $ndt| {
                            let _ = &$types;

                            // TODO: This should be doable in the macro instead of the runtime. This will do for now though.
                            let (type_name, module_path) = {
                                let s = stringify!($type_path);
                                let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
                                 let s = cleaned.split('<').next().unwrap();
                                 if let Some((path, name)) = s.rsplit_once("::") {
                                     (name.to_string(), path.to_string())
                                 } else {
                                     (s.to_string(), String::new())
                                 }
                            };

                            $ndt.set_name(::std::borrow::Cow::Owned(type_name));
                            $ndt.set_module_path(::std::borrow::Cow::Owned(module_path));

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
