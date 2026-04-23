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

macro_rules! _impl_ndt {
    // Public API
    // inline the type. Types which shouldn't show up as named in the output. Eg. `String`
    ($head:ident :: $( $tail:ident )::+ $(< $( $generic:ident ),* >)? as $as_ty:ty = inline) => {
        impl_ndt!(true, false $head :: $( $tail )::+ $(< $( $generic ),* >)? as $as_ty);
    };
    // inline the type, and passthrough `#[specta(inline)]`. Used for containers like `Box<T>`
    ($head:ident :: $( $tail:ident )::+ $(< $( $generic:ident ),* >)? as $as_ty:ty = inline_passthrough) => {
        impl_ndt!(true, true $head :: $( $tail )::+ $(< $( $generic ),* >)? as $as_ty);
    };
    // Used for named types. Like `std::ops::Range`.
    ($head:ident :: $( $tail:ident )::+ $(< $( $generic:ident ),* >)? as $as_ty:ty = named) => {
        impl_ndt!(false, false $head :: $( $tail )::+ $(< $( $generic ),* >)? as $as_ty);
    };

    // Base implementation
    ($inline:literal, $container:literal $head:ident :: $( $tail:ident )::+ $(< $( $generic:ident ),* >)? as $as_ty:ty) => {
        fn definition(types: &mut Types) -> DataType {
            static SENTINEL: &str = stringify!($head::$( $tail )::+ $(< $( $generic ),* >)?);
            static GENERICS: &[datatype::GenericDefinition] = &[
                $($(
                    datatype::GenericDefinition::new(
                        ::std::borrow::Cow::Borrowed(stringify!($generic)),
                        None,
                    ),
                )*)?
            ];
            DataType::Reference(datatype::NamedDataType::init_with_sentinel(
                SENTINEL,
                GENERICS,
                &[
                    $($(
                        (
                            datatype::Generic::new(::std::borrow::Cow::Borrowed(stringify!($generic))),
                            <$generic as Type>::definition(types),
                        ),
                    )*)?
                ],
                false,
                $inline,
                $container,
                types,
                |types, ndt| {
                    ndt.name = ::std::borrow::Cow::Borrowed(
                        impl_ndt!(@type_name $head :: $( $tail )::+ $(< $( $generic ),* >)?)
                    );
                    ndt.module_path = ::std::borrow::Cow::Borrowed(
                        impl_ndt!(@module_path $head :: $( $tail )::+)
                    );
                    if !$inline {
                        $($(
                            #[allow(dead_code)]
                            pub(crate) struct $generic;
                            impl Type for $generic {
                                fn definition(_: &mut Types) -> DataType {
                                    datatype::Generic::new(
                                        ::std::borrow::Cow::Borrowed(stringify!($generic))
                                    ).into()
                                }
                            }
                        )*)?
                        ndt.ty = Some(<$as_ty as Type>::definition(types));
                    }
                },
                |types| <$as_ty as Type>::definition(types),
            ))
        }
    };

    // Helpers for determining NDT name
    (@type_name $name:ident) => {
        stringify!($name)
    };
    (@type_name $head:ident :: $( $tail:ident )::+ $(< $( $generic:ident ),* >)?) => {
        impl_ndt!(@type_name $( $tail )::+ $(< $( $generic ),* >)?)
    };
    (@type_name $name:ident < $generic:ident $(, $rest:ident)* >) => {
        concat!(
            stringify!($name),
            "<",
            stringify!($generic)
            $(, ", ", stringify!($rest))*
            ,
            ">"
        )
    };

    // Helpers for determining NDT module path
    (@module_path $head:ident :: $name:ident) => {
        stringify!($head)
    };
    (@module_path $head:ident :: $next:ident :: $( $tail:ident )::+) => {
        concat!(
            stringify!($head),
            "::",
            impl_ndt!(@module_path $next :: $( $tail )::+)
        )
    };
}

pub(crate) use _impl_ndt as impl_ndt;
pub(crate) use _impl_primitives as impl_primitives;
pub(crate) use _impl_tuple as impl_tuple;
