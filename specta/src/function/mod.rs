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
#[doc(hidden)]
#[macro_export]
macro_rules! _fn_datatype {
    ($function:path) => {
        $crate::internal::paste! { [<__specta__fn__ $function>]!(@export_fn; $function) }
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
    ($($command:path),* $(,)?) => {{
        fn export(type_map: &mut $crate::TypeMap) -> Vec<$crate::datatype::Function> {
            vec![$($crate::fn_datatype!($command)(type_map)),*]
        }

        export
    }};
}

#[doc(inline)]
pub use _collect_functions as collect_functions;
#[doc(inline)]
pub use _fn_datatype as fn_datatype;
