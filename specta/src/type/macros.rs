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
    () => {};

    // Multiple types
    (
        $module_path:literal $ty:ident
        $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)?
        $( where { $($bounds:tt)* } )?
        as $as_ty:ty = $kind:ident;
        $($rest:tt)*
    ) => {
        impl_ndt!(@single
            $module_path $ty
            $(< $( $lifetime, )* $( $generic ),* >)?
            $( where { $($bounds)* } )?
            as $as_ty = $kind
        );
        impl_ndt!($($rest)*);
    };
    (
        $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? >
        < $( $impl_generic:ident ),+ $(,)? >
        $( where { $($bounds:tt)* } )?
        as $as_ty:ty = $kind:ident;
        $($rest:tt)*
    ) => {
        impl_ndt!(@single
            $head :: $( $tail )::+ < $( $specta_generic ),* >
            < $( $impl_generic ),* >
            $( where { $($bounds)* } )?
            as $as_ty = $kind
        );
        impl_ndt!($($rest)*);
    };
    (
        $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? >
        < $first_impl_generic:ident, $second_impl_generic:ident, $third_impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+ $(,)? >
        $( where { $($bounds:tt)* } )?
        as $as_ty:ty = $kind:ident;
        $($rest:tt)*
    ) => {
        impl_ndt!(@single
            $head :: $( $tail )::+ < $( $specta_generic ),* >
            < $first_impl_generic, $second_impl_generic, $third_impl_generic, $( const $const_generic : $const_ty ),* >
            $( where { $($bounds)* } )?
            as $as_ty = $kind
        );
        impl_ndt!($($rest)*);
    };
    (
        $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? >
        < $first_impl_generic:ident, $second_impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+ $(,)? >
        $( where { $($bounds:tt)* } )?
        as $as_ty:ty = $kind:ident;
        $($rest:tt)*
    ) => {
        impl_ndt!(@single
            $head :: $( $tail )::+ < $( $specta_generic ),* >
            < $first_impl_generic, $second_impl_generic, $( const $const_generic : $const_ty ),* >
            $( where { $($bounds)* } )?
            as $as_ty = $kind
        );
        impl_ndt!($($rest)*);
    };
    (
        $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? >
        < $first_impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+, $( $rest_impl_generic:ident ),+ $(,)? >
        $( where { $($bounds:tt)* } )?
        as $as_ty:ty = $kind:ident;
        $($rest:tt)*
    ) => {
        impl_ndt!(@single
            $head :: $( $tail )::+ < $( $specta_generic ),* >
            < $first_impl_generic, $( const $const_generic : $const_ty, )* $( $rest_impl_generic ),* >
            $( where { $($bounds)* } )?
            as $as_ty = $kind
        );
        impl_ndt!($($rest)*);
    };
    (
        $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? >
        < $impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+ $(,)? >
        $( where { $($bounds:tt)* } )?
        as $as_ty:ty = $kind:ident;
        $($rest:tt)*
    ) => {
        impl_ndt!(@single
            $head :: $( $tail )::+ < $( $specta_generic ),* >
            < $impl_generic, $( const $const_generic : $const_ty ),* >
            $( where { $($bounds)* } )?
            as $as_ty = $kind
        );
        impl_ndt!($($rest)*);
    };
    (
        $head:ident :: $( $tail:ident )::+ < $( const $const_generic:ident : $const_ty:ty ),+ $(,)? >
        $( where { $($bounds:tt)* } )?
        as $as_ty:ty = $kind:ident;
        $($rest:tt)*
    ) => {
        impl_ndt!(@single
            $head :: $( $tail )::+ < $( const $const_generic : $const_ty ),* >
            $( where { $($bounds)* } )?
            as $as_ty = $kind
        );
        impl_ndt!($($rest)*);
    };
    (
        $head:ident :: $( $tail:ident )::+ < $( const $const_generic:ident : $const_ty:ty ),+, $( $rest_impl_generic:ident ),+ $(,)? >
        $( where { $($bounds:tt)* } )?
        as $as_ty:ty = $kind:ident;
        $($rest:tt)*
    ) => {
        impl_ndt!(@single
            $head :: $( $tail )::+ < $( const $const_generic : $const_ty, )* $( $rest_impl_generic ),* >
            $( where { $($bounds)* } )?
            as $as_ty = $kind
        );
        impl_ndt!($($rest)*);
    };
    (
        $head:ident :: $( $tail:ident )::+ < $impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+ $(,)? >
        $( where { $($bounds:tt)* } )?
        as $as_ty:ty = $kind:ident;
        $($rest:tt)*
    ) => {
        impl_ndt!(@single
            $head :: $( $tail )::+ < $impl_generic, $( const $const_generic : $const_ty ),* >
            $( where { $($bounds)* } )?
            as $as_ty = $kind
        );
        impl_ndt!($($rest)*);
    };
    (
        $(
            $head:ident :: $( $tail:ident )::+
            $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)?
            $( where { $($bounds:tt)* } )?
            as $as_ty:ty = $kind:ident
        );+ $(;)?
    ) => {
        $(
            impl_ndt!(@single
                $head :: $( $tail )::+
                $(< $( $lifetime, )* $( $generic ),* >)?
                $( where { $($bounds)* } )?
                as $as_ty = $kind
            );
        )+
    };

    // Single type
    (@single $module_path:literal $ty:ident $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)? $( where { $($bounds:tt)* } )? as $as_ty:ty = inline) => {
        impl_ndt!(@kind inline $module_path $ty $(< $( $lifetime, )* $( $generic ),* >)? $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $module_path:literal $ty:ident $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)? $( where { $($bounds:tt)* } )? as $as_ty:ty = passthrough) => {
        impl_ndt!(@kind passthrough $module_path $ty $(< $( $lifetime, )* $( $generic ),* >)? $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $module_path:literal $ty:ident $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)? $( where { $($bounds:tt)* } )? as $as_ty:ty = named) => {
        impl_ndt!(@kind named $module_path $ty $(< $( $lifetime, )* $( $generic ),* >)? $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? > < $( $impl_generic:ident ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = inline) => {
        impl_ndt!(true, false [$( $specta_generic ),*] [$( $impl_generic, )*] [$head :: $( $tail )::+] [< $( $specta_generic ),* >] [< $( $impl_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? > < $( $impl_generic:ident ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = passthrough) => {
        impl_ndt!(false, true [$( $specta_generic ),*] [$( $impl_generic, )*] [$head :: $( $tail )::+] [< $( $specta_generic ),* >] [< $( $impl_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? > < $( $impl_generic:ident ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = named) => {
        impl_ndt!(false, false [$( $specta_generic ),*] [$( $impl_generic, )*] [$head :: $( $tail )::+] [< $( $specta_generic ),* >] [< $( $impl_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? > < $first_impl_generic:ident, $second_impl_generic:ident, $third_impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = inline) => {
        impl_ndt!(true, false [$( $specta_generic ),*] [$first_impl_generic, $second_impl_generic, $third_impl_generic, $( const $const_generic : $const_ty, )*] [$head :: $( $tail )::+] [< $( $specta_generic ),* >] [< $first_impl_generic, $second_impl_generic, $third_impl_generic, $( $const_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? > < $first_impl_generic:ident, $second_impl_generic:ident, $third_impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = passthrough) => {
        impl_ndt!(false, true [$( $specta_generic ),*] [$first_impl_generic, $second_impl_generic, $third_impl_generic, $( const $const_generic : $const_ty, )*] [$head :: $( $tail )::+] [< $( $specta_generic ),* >] [< $first_impl_generic, $second_impl_generic, $third_impl_generic, $( $const_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? > < $first_impl_generic:ident, $second_impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = inline) => {
        impl_ndt!(true, false [$( $specta_generic ),*] [$first_impl_generic, $second_impl_generic, $( const $const_generic : $const_ty, )*] [$head :: $( $tail )::+] [< $( $specta_generic ),* >] [< $first_impl_generic, $second_impl_generic, $( $const_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? > < $first_impl_generic:ident, $second_impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = passthrough) => {
        impl_ndt!(false, true [$( $specta_generic ),*] [$first_impl_generic, $second_impl_generic, $( const $const_generic : $const_ty, )*] [$head :: $( $tail )::+] [< $( $specta_generic ),* >] [< $first_impl_generic, $second_impl_generic, $( $const_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? > < $first_impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+, $( $rest_impl_generic:ident ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = inline) => {
        impl_ndt!(true, false [$( $specta_generic ),*] [$first_impl_generic, $( const $const_generic : $const_ty, )* $( $rest_impl_generic, )*] [$head :: $( $tail )::+] [< $( $specta_generic ),* >] [< $first_impl_generic, $( $const_generic, )* $( $rest_impl_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? > < $first_impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+, $( $rest_impl_generic:ident ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = passthrough) => {
        impl_ndt!(false, true [$( $specta_generic ),*] [$first_impl_generic, $( const $const_generic : $const_ty, )* $( $rest_impl_generic, )*] [$head :: $( $tail )::+] [< $( $specta_generic ),* >] [< $first_impl_generic, $( $const_generic, )* $( $rest_impl_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? > < $impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = inline) => {
        impl_ndt!(true, false [$( $specta_generic ),*] [$impl_generic, $( const $const_generic : $const_ty, )*] [$head :: $( $tail )::+] [< $( $specta_generic ),* >] [< $impl_generic, $( $const_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( $specta_generic:ident ),* $(,)? > < $impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = passthrough) => {
        impl_ndt!(false, true [$( $specta_generic ),*] [$impl_generic, $( const $const_generic : $const_ty, )*] [$head :: $( $tail )::+] [< $( $specta_generic ),* >] [< $impl_generic, $( $const_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( const $const_generic:ident : $const_ty:ty ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = inline) => {
        impl_ndt!(true, false [] [$( const $const_generic : $const_ty, )*] [$head :: $( $tail )::+] [] [< $( $const_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( const $const_generic:ident : $const_ty:ty ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = passthrough) => {
        impl_ndt!(false, true [] [$( const $const_generic : $const_ty, )*] [$head :: $( $tail )::+] [] [< $( $const_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( const $const_generic:ident : $const_ty:ty ),+, $( $rest_impl_generic:ident ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = inline) => {
        impl_ndt!(true, false [] [$( const $const_generic : $const_ty, )* $( $rest_impl_generic, )*] [$head :: $( $tail )::+] [] [< $( $const_generic, )* $( $rest_impl_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $( const $const_generic:ident : $const_ty:ty ),+, $( $rest_impl_generic:ident ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = passthrough) => {
        impl_ndt!(false, true [] [$( const $const_generic : $const_ty, )* $( $rest_impl_generic, )*] [$head :: $( $tail )::+] [] [< $( $const_generic, )* $( $rest_impl_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = inline) => {
        impl_ndt!(true, false [$impl_generic] [$impl_generic, $( const $const_generic : $const_ty, )*] [$head :: $( $tail )::+] [< $impl_generic >] [< $impl_generic, $( $const_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ < $impl_generic:ident, $( const $const_generic:ident : $const_ty:ty ),+ $(,)? > $( where { $($bounds:tt)* } )? as $as_ty:ty = passthrough) => {
        impl_ndt!(false, true [$impl_generic] [$impl_generic, $( const $const_generic : $const_ty, )*] [$head :: $( $tail )::+] [< $impl_generic >] [< $impl_generic, $( $const_generic ),* >] $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)? $( where { $($bounds:tt)* } )? as $as_ty:ty = inline) => {
        impl_ndt!(@kind inline $head :: $( $tail )::+ $(< $( $lifetime, )* $( $generic ),* >)? $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)? $( where { $($bounds:tt)* } )? as $as_ty:ty = passthrough) => {
        impl_ndt!(@kind passthrough $head :: $( $tail )::+ $(< $( $lifetime, )* $( $generic ),* >)? $( where { $($bounds)* } )? as $as_ty);
    };
    (@single $head:ident :: $( $tail:ident )::+ $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)? $( where { $($bounds:tt)* } )? as $as_ty:ty = named) => {
        impl_ndt!(@kind named $head :: $( $tail )::+ $(< $( $lifetime, )* $( $generic ),* >)? $( where { $($bounds)* } )? as $as_ty);
    };

    (@kind inline $($tokens:tt)*) => {
        impl_ndt!(true, false $($tokens)*);
    };
    (@kind passthrough $($tokens:tt)*) => {
        impl_ndt!(false, true $($tokens)*);
    };
    (@kind named $($tokens:tt)*) => {
        impl_ndt!(false, false $($tokens)*);
    };

    // Base implementation. Providing a where clause opts out of automatic Type bounds so
    // callers can use non-Specta impl generics.
    ($inline:literal, $container:literal $module_path:literal $ty:ident $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)? where { $($bounds:tt)* } as $as_ty:ty) => {
        impl<$( $( $lifetime, )* $( $generic ),* )?> Type for $ty $(< $( $lifetime, )* $( $generic ),* >)?
        where
            $($bounds)*
        {
            fn definition(types: &mut Types) -> DataType {
                use $crate::datatype;

                impl_ndt!(@definition_body false stringify!($ty $(< $( $generic ),* >)?), [$( $( $generic ),* )?], [$ty $(< $( $generic ),* >)?], $module_path, $inline, $container, $as_ty, types)
            }
        }
    };
    ($inline:literal, $container:literal $module_path:literal $ty:ident $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)? as $as_ty:ty) => {
        impl<$( $( $lifetime, )* $( $generic ),* )?> Type for $ty $(< $( $lifetime, )* $( $generic ),* >)?
        where
            $(
                $( $generic: Type, )*
            )?
        {
            fn definition(types: &mut Types) -> DataType {
                use $crate::datatype;

                impl_ndt!(@definition_body true stringify!($ty $(< $( $generic ),* >)?), [$( $( $generic ),* )?], [$ty $(< $( $generic ),* >)?], $module_path, $inline, $container, $as_ty, types)
            }
        }
    };
    ($inline:literal, $container:literal $head:ident :: $( $tail:ident )::+ $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)? where { $($bounds:tt)* } as $as_ty:ty) => {
        impl<$( $( $lifetime, )* $( $generic ),* )?> Type for $head :: $( $tail )::+ $(< $( $lifetime, )* $( $generic ),* >)?
        where
            $($bounds)*
        {
            fn definition(types: &mut Types) -> DataType {
                use $crate::datatype;

                impl_ndt!(@definition_body false stringify!($head::$( $tail )::+ $(< $( $generic ),* >)?), [$( $( $generic ),* )?], [$head :: $( $tail )::+ $(< $( $generic ),* >)?], impl_ndt!(@module_path $head :: $( $tail )::+), $inline, $container, $as_ty, types)
            }
        }
    };
    ($inline:literal, $container:literal $head:ident :: $( $tail:ident )::+ $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)? as $as_ty:ty) => {
        impl<$( $( $lifetime, )* $( $generic ),* )?> Type for $head :: $( $tail )::+ $(< $( $lifetime, )* $( $generic ),* >)?
        where
            $(
                $( $generic: Type, )*
            )?
        {
            fn definition(types: &mut Types) -> DataType {
                use $crate::datatype;

                impl_ndt!(@definition_body true stringify!($head::$( $tail )::+ $(< $( $generic ),* >)?), [$( $( $generic ),* )?], [$head :: $( $tail )::+ $(< $( $generic ),* >)?], impl_ndt!(@module_path $head :: $( $tail )::+), $inline, $container, $as_ty, types)
            }
        }
    };

    // Base implementation for types with const generics. Const generics are part of the Rust
    // implementation but not Specta generic definitions.
    ($inline:literal, $container:literal [$($generic:ident),*] [$($impl_generics:tt)*] [$($ty:tt)*] [$($specta_generics:tt)*] [$($ty_generics:tt)*] where { $($bounds:tt)* } as $as_ty:ty) => {
        impl<$($impl_generics)*> Type for $($ty)* $($ty_generics)*
        where
            $($bounds)*
        {
            fn definition(types: &mut Types) -> DataType {
                use $crate::datatype;

                impl_ndt!(@definition_body false stringify!($($ty)* $($ty_generics)*), [$($generic),*], [$($ty)* $($specta_generics)*], impl_ndt!(@module_path $($ty)*), $inline, $container, $as_ty, types)
            }
        }
    };
    ($inline:literal, $container:literal [$($generic:ident),*] [$($impl_generics:tt)*] [$($ty:tt)*] [$($specta_generics:tt)*] [$($ty_generics:tt)*] as $as_ty:ty) => {
        impl<$($impl_generics)*> Type for $($ty)* $($ty_generics)*
        where
            $($generic: Type,)*
        {
            fn definition(types: &mut Types) -> DataType {
                use $crate::datatype;

                impl_ndt!(@definition_body true stringify!($($ty)* $($ty_generics)*), [$($generic),*], [$($ty)* $($specta_generics)*], impl_ndt!(@module_path $($ty)*), $inline, $container, $as_ty, types)
            }
        }
    };

    // Helpers for determining NDT name
    (@definition_body $typed_generics:tt $sentinel:expr, [$($generic:ident),*], [$($type_name:tt)*], $module_path:expr, $inline:literal, $container:literal, $as_ty:ty, $types:ident) => {{
        static SENTINEL: &str = $sentinel;
        static GENERICS: &[datatype::GenericDefinition] = &[
            $(
                datatype::GenericDefinition::new(
                    ::std::borrow::Cow::Borrowed(stringify!($generic)),
                    None,
                ),
            )*
        ];

        let definition = |types: &mut Types| {
            DataType::Reference(datatype::NamedDataType::init_with_sentinel(
                SENTINEL,
                &[
                    $(
                        (
                            datatype::Generic::new(::std::borrow::Cow::Borrowed(stringify!($generic))),
                            impl_ndt!(@generic_dt $typed_generics $generic types),
                        ),
                    )*
                ],
                false,
                $inline,
                $container,
                types,
                |types, ndt| {
                    ndt.name = ::std::borrow::Cow::Borrowed(impl_ndt!(@type_name $($type_name)*));
                    ndt.module_path = ::std::borrow::Cow::Borrowed($module_path);
                    ndt.generics = ::std::borrow::Cow::Borrowed(GENERICS);
                    if !$inline {
                        $(
                            #[allow(dead_code)]
                            pub(crate) struct $generic;
                            impl Type for $generic {
                                fn definition(_: &mut Types) -> DataType {
                                    datatype::Generic::new(
                                        ::std::borrow::Cow::Borrowed(stringify!($generic))
                                    ).into()
                                }
                            }
                        )*
                        ndt.ty = Some(<$as_ty as Type>::definition(types));
                    }
                },
                |types| <$as_ty as Type>::definition(types),
            ))
        };

        if $inline {
            datatype::inline($types, definition)
        } else {
            definition($types)
        }
    }};
    (@generic_dt true $generic:ident $types:ident) => {
        <$generic as Type>::definition($types)
    };
    (@generic_dt false $generic:ident $types:ident) => {
        datatype::Generic::new(::std::borrow::Cow::Borrowed(stringify!($generic))).into()
    };

    // Helpers for determining NDT name
    (@type_name $head:ident :: $( $tail:ident )::+ $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)?) => {
        impl_ndt!(@type_name $( $tail )::+ $(< $( $lifetime, )* $( $generic ),* >)?)
    };
    (@type_name $name:ident $(< $( $lifetime:lifetime, )* $( $generic:ident ),* $(,)? >)?) => {
        stringify!($name)
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

#[allow(unused_imports)]
pub(crate) use _impl_ndt as impl_ndt;
pub(crate) use _impl_primitives as impl_primitives;
pub(crate) use _impl_tuple as impl_tuple;
