mod result;

use std::borrow::Cow;

// TODO: Not pub
pub use result::*;

use crate::*;

/// Returns a [`FunctionDataType`] for a given function that has been annotated with
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
///     let typ = fn_datatype!(some_function);
///
///     assert_eq!(typ.name, "some_function");
///     assert_eq!(typ.args.len(), 2);
///     assert_eq!(typ.result, Some(DataType::Primitive(PrimitiveType::bool)));
/// }
/// ```
#[macro_export]
macro_rules! fn_datatype {
    ($function:path) => {{
        let type_map = &mut $crate::TypeMap::default();
        specta::internal::internal_fn_datatype!($function);
        result
    }};
    ($type_map:ident; $function:path) => {{
        let type_map: &mut $crate::TypeMap = &mut $type_map;
        specta::internal::internal_fn_datatype!($function);
        result
    }};
}

/// Contains type information about a function annotated with [`specta`](macro@crate::specta).
/// Returned by [`fn_datatype`].
#[derive(Debug, Clone)]
pub struct FunctionDataType {
    /// Whether the function is async.
    pub asyncness: bool,
    /// The function's name.
    pub name: Cow<'static, str>,
    /// The name and type of each of the function's arguments.
    pub args: Vec<(Cow<'static, str>, DataType)>,
    /// The return type of the function.
    pub result: Option<DataType>,
    /// The function's documentation. Detects both `///` and `#[doc = ...]` style documentation.
    pub docs: Cow<'static, str>,
    /// The deprecated status of the function.
    pub deprecated: Option<DeprecatedType>,
}

/// Collects function types into a [`Vec`],
/// and all downstream types into a [`TypeMap`] instance.
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
///     // `type_defs` is created internally
///     let (functions, type_defs) = functions::collect_functions![some_function];
///
///     let custom_type_defs = TypeMap::default();
///
///     // `type_defs` is provided.
///     // This can be used when integrating multiple specta-enabled libraries.
///     let (functions, custom_type_defs) = functions::collect_functions![
///         custom_type_defs; // You can provide a custom map to collect the types into
///         some_function
///     ];
/// }
/// ````
#[macro_export]
macro_rules! collect_functions {
    ($type_map:ident; $($command:path),* $(,)?) => {{
        let mut type_map: $crate::TypeMap = $type_map;
        ([$($crate::fn_datatype!(type_map; $command)),*]
            .into_iter()
            .collect::<Vec<_>>(), type_map)
    }};
    ($($command:path),* $(,)?) => {{
        let mut type_map = $crate::TypeMap::default();
        $crate::functions::collect_functions!(type_map; $($command),*)
    }};
}

pub type CollectFunctionsResult = (Vec<FunctionDataType>, TypeMap);

pub use crate::collect_functions;
