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
/// # Recursion limit reached while expanding the macro `fn_datatype`
///
/// This macro requires recursion internally to correctly function so you may run into the recursion limit. From my testing you can have 31 path segments before you hit the recursion limit. The size of the segment or the amount of generics in the segment should not affect this limit.
///
/// If your having issues with this limit you can increase your [`recursion_limit`](https://doc.rust-lang.org/reference/attributes/limits.html#the-recursion_limit-attribute) by adding `#![recursion_limit = "1024"]` to your `main.rs`. If your able to hit this limit in other scenarios please [let us know](https://github.com/oscartbeaumont/tauri-specta/issues/114) and we can apply some potential optimizations.
///
#[doc(hidden)]
#[macro_export]
macro_rules! _fn_datatype {
    // Hide distracting implementation details from the generated rustdoc.
    ($($json:tt)*) => {
        $crate::function::_fn_datatype_internal!($($json)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _fn_datatype_internal {
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
    () => {{
        fn export(_: &mut $crate::TypeMap) -> Vec<$crate::datatype::Function> {
            vec![]
        }

        export
    }};
    ($($b:tt $(:: $($p:ident)? $(<$g:path>)? )* ),*) => {{
        fn export(type_map: &mut $crate::TypeMap) -> Vec<$crate::datatype::Function> {
            vec![
                $($crate::function::fn_datatype!($b $($(::$p)? $(::<$g>)? )* )(type_map)),*
            ]
        }

        export
    }};
}

#[doc(inline)]
pub use _collect_functions as collect_functions;
#[doc(inline)]
pub use _fn_datatype as fn_datatype;
#[doc(hidden)]
pub use _fn_datatype_internal;
