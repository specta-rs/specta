//! Support for exporting Rust function.
//!
//! TODO: Docs. Talk about how Specta doesn't export functions but it helps you to.

mod arg;
mod result;
mod specta_fn;

pub use arg::FunctionArg;
pub use result::FunctionResult;
#[doc(hidden)]
pub use result::{FunctionResultFutureMarker, FunctionResultMarker};
pub(crate) use specta_fn::SpectaFn;

/// Returns a [`Function`](crate::datatype::Function) for a given function that has been annotated with
/// [`specta`](macro@crate::specta).
///
/// # Examples
///
/// ```rust
/// use specta::*;
///
/// #[specta]
/// fn some_function(name: String, age: i32) -> bool {
///     true
/// }
///
/// fn main() {
///     let typ = fn_datatype!(some_function)(&mut TypeMap::default())
///
///     assert_eq!(typ.name, "some_function");
///     assert_eq!(typ.args.len(), 2);
///     assert_eq!(typ.result, Some(DataType::Primitive(PrimitiveType::bool)));
/// }
/// ```
///
#[doc(hidden)]
#[macro_export]
macro_rules! _fn_datatype {
    // Hide distracting implementation details from the generated rustdoc.
    ($($json:tt)*) => {
        $crate::function::_fn_datatype_inner!($($json)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _fn_datatype_inner {
    ([$($path:tt)*] [$($full:tt)*] [$last:tt]) => {
        $crate::internal::paste! {
            $($path)* [<__specta__fn__ $last>]!(@export_fn; $($full)*)
        }
    };
    ([$($path:tt)*] [$($full:tt)*] [$($last:tt)?] $t:tt :: <$($g:path)*> $($rest:tt)*) => {
        $crate::function::fn_datatype!([$($path)* $($last)*] [$($full)* $t::<$($g)*>] [$t] $($rest)*)
    };
    ([$($path:tt)*] [$($full:tt)*] [$($last:tt)?] $t:tt $($rest:tt)*) => {
        $crate::function::fn_datatype!([$($path)* $($last)*] [$($full)* $t] [$t] $($rest)*)
    };
    () => {{
            compile_error!("fn_datatype must be provided a function path as an argument");
    }};
    ($($rest:tt)*) => {
        $crate::function::fn_datatype!([] [] [] $($rest)*)
    };
}

/// Collects function types into a [`Vec`],
/// and all downstream types into a [`TypeMap`](crate::TypeMap) instance.
///
/// Specifying a `type_map` argument allows a custom [`TypeMap`] to be used.
///
/// # Examples
///
/// ```rust
/// use specta::*;
///
/// #[specta]
/// fn some_function(name: String, age: i32) -> bool {
///     true
/// }
///
/// fn main() {
///     let functions = function::collect_functions![some_function](&mut TypeMap::default());
/// }
/// ````
#[doc(hidden)]
#[macro_export]
macro_rules! _collect_functions {
    // Hide distracting implementation details from the generated rustdoc.
    ($($json:tt)*) => {
        $crate::function::_collect_functions_inner!($($json)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _collect_functions_inner {
    ($tm:ident [] [$($result:expr)*]) => {{
        fn export($tm: &mut $crate::TypeMap) -> Vec<$crate::datatype::Function> {
            vec![$($result),*]
        }

        export
    }};
    ($tm:ident [$($parts:tt)*] [$($result:expr)*]) => {
        $crate::function::collect_functions!($tm [] [$($result)* $crate::function::fn_datatype!($($parts)*)($tm)])
    };
    ($tm:ident [$($parts:tt)*] [$($result:expr)*] , $($rest:tt)*) => {
        $crate::function::collect_functions!($tm [] [$($result)* $crate::function::fn_datatype!($($parts)*)($tm)] $($rest)*)
    };
    ($tm:ident [$($parts:tt)*] [$($result:expr)*] $t:tt $($rest:tt)*) => {
        $crate::function::collect_functions!($tm [$($parts)* $t] [$($result)*] $($rest)*)
    };
    ($($command:tt)*) => {
        $crate::function::collect_functions!(type_map [] [] $($command)*)
    };
}

#[doc(inline)]
pub use _collect_functions as collect_functions;
#[doc(hidden)]
pub use _collect_functions_inner;
#[doc(inline)]
pub use _fn_datatype as fn_datatype;
#[doc(hidden)]
pub use _fn_datatype_inner;
