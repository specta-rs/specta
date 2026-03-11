pub use crate::inflection::RenameRule;

/// Conversion metadata parsed from serde conversion attributes.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConversionType {
    pub type_src: String,
}

/// Parsed serde container attributes.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SerdeContainerAttrs {
    pub rename_serialize: Option<String>,
    pub rename_deserialize: Option<String>,
    pub rename_all_serialize: Option<RenameRule>,
    pub rename_all_deserialize: Option<RenameRule>,
    pub rename_all_fields_serialize: Option<RenameRule>,
    pub rename_all_fields_deserialize: Option<RenameRule>,
    pub deny_unknown_fields: bool,
    pub tag: Option<String>,
    pub content: Option<String>,
    pub untagged: bool,
    pub default: Option<String>,
    pub remote: Option<String>,
    pub transparent: bool,
    pub from: Option<ConversionType>,
    pub try_from: Option<ConversionType>,
    pub into: Option<ConversionType>,
    pub resolved_from: Option<specta::datatype::DataType>,
    pub resolved_try_from: Option<specta::datatype::DataType>,
    pub resolved_into: Option<specta::datatype::DataType>,
    pub serde_crate: Option<String>,
    pub expecting: Option<String>,
    pub variant_identifier: bool,
    pub field_identifier: bool,
}

/// Parsed serde variant attributes.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SerdeVariantAttrs {
    pub rename_serialize: Option<String>,
    pub rename_deserialize: Option<String>,
    pub aliases: Vec<String>,
    pub rename_all_serialize: Option<RenameRule>,
    pub rename_all_deserialize: Option<RenameRule>,
    pub skip_serializing: bool,
    pub skip_deserializing: bool,
    pub serialize_with: Option<String>,
    pub deserialize_with: Option<String>,
    pub with: Option<String>,
    pub borrow: Option<String>,
    pub other: bool,
    pub untagged: bool,
}

/// Parsed serde field attributes.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SerdeFieldAttrs {
    pub rename_serialize: Option<String>,
    pub rename_deserialize: Option<String>,
    pub aliases: Vec<String>,
    pub default: Option<String>,
    pub flatten: bool,
    pub skip_serializing: bool,
    pub skip_deserializing: bool,
    pub skip_serializing_if: Option<String>,
    pub serialize_with: Option<String>,
    pub deserialize_with: Option<String>,
    pub with: Option<String>,
    pub borrow: Option<String>,
    pub getter: Option<String>,
}

#[doc(hidden)]
pub fn merge_container_attrs(target: &mut SerdeContainerAttrs, other: SerdeContainerAttrs) {
    if other.rename_serialize.is_some() {
        target.rename_serialize = other.rename_serialize;
    }
    if other.rename_deserialize.is_some() {
        target.rename_deserialize = other.rename_deserialize;
    }
    if other.rename_all_serialize.is_some() {
        target.rename_all_serialize = other.rename_all_serialize;
    }
    if other.rename_all_deserialize.is_some() {
        target.rename_all_deserialize = other.rename_all_deserialize;
    }
    if other.rename_all_fields_serialize.is_some() {
        target.rename_all_fields_serialize = other.rename_all_fields_serialize;
    }
    if other.rename_all_fields_deserialize.is_some() {
        target.rename_all_fields_deserialize = other.rename_all_fields_deserialize;
    }
    target.deny_unknown_fields |= other.deny_unknown_fields;
    if other.tag.is_some() {
        target.tag = other.tag;
    }
    if other.content.is_some() {
        target.content = other.content;
    }
    target.untagged |= other.untagged;
    if other.default.is_some() {
        target.default = other.default;
    }
    if other.remote.is_some() {
        target.remote = other.remote;
    }
    target.transparent |= other.transparent;
    if other.from.is_some() {
        target.from = other.from;
    }
    if other.try_from.is_some() {
        target.try_from = other.try_from;
    }
    if other.into.is_some() {
        target.into = other.into;
    }
    if other.resolved_from.is_some() {
        target.resolved_from = other.resolved_from;
    }
    if other.resolved_try_from.is_some() {
        target.resolved_try_from = other.resolved_try_from;
    }
    if other.resolved_into.is_some() {
        target.resolved_into = other.resolved_into;
    }
    if other.serde_crate.is_some() {
        target.serde_crate = other.serde_crate;
    }
    if other.expecting.is_some() {
        target.expecting = other.expecting;
    }
    target.variant_identifier |= other.variant_identifier;
    target.field_identifier |= other.field_identifier;
}

#[doc(hidden)]
pub fn merge_variant_attrs(target: &mut SerdeVariantAttrs, other: SerdeVariantAttrs) {
    if other.rename_serialize.is_some() {
        target.rename_serialize = other.rename_serialize;
    }
    if other.rename_deserialize.is_some() {
        target.rename_deserialize = other.rename_deserialize;
    }
    target.aliases.extend(other.aliases);
    if other.rename_all_serialize.is_some() {
        target.rename_all_serialize = other.rename_all_serialize;
    }
    if other.rename_all_deserialize.is_some() {
        target.rename_all_deserialize = other.rename_all_deserialize;
    }
    target.skip_serializing |= other.skip_serializing;
    target.skip_deserializing |= other.skip_deserializing;
    if other.serialize_with.is_some() {
        target.serialize_with = other.serialize_with;
    }
    if other.deserialize_with.is_some() {
        target.deserialize_with = other.deserialize_with;
    }
    if other.with.is_some() {
        target.with = other.with;
    }
    if other.borrow.is_some() {
        target.borrow = other.borrow;
    }
    target.other |= other.other;
    target.untagged |= other.untagged;
}

#[doc(hidden)]
pub fn merge_field_attrs(target: &mut SerdeFieldAttrs, other: SerdeFieldAttrs) {
    if other.rename_serialize.is_some() {
        target.rename_serialize = other.rename_serialize;
    }
    if other.rename_deserialize.is_some() {
        target.rename_deserialize = other.rename_deserialize;
    }
    target.aliases.extend(other.aliases);
    if other.default.is_some() {
        target.default = other.default;
    }
    target.flatten |= other.flatten;
    target.skip_serializing |= other.skip_serializing;
    target.skip_deserializing |= other.skip_deserializing;
    if other.skip_serializing_if.is_some() {
        target.skip_serializing_if = other.skip_serializing_if;
    }
    if other.serialize_with.is_some() {
        target.serialize_with = other.serialize_with;
    }
    if other.deserialize_with.is_some() {
        target.deserialize_with = other.deserialize_with;
    }
    if other.with.is_some() {
        target.with = other.with;
    }
    if other.borrow.is_some() {
        target.borrow = other.borrow;
    }
    if other.getter.is_some() {
        target.getter = other.getter;
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __specta_serde_set_str {
    ($target:expr, $field:ident, $value:literal) => {{
        const _: &str = $value;
        $target.$field = Some(String::from($value));
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __specta_serde_set_case {
    ($target:expr, $field:ident, "lowercase") => {{
        $target.$field = Some($crate::RenameRule::LowerCase);
    }};
    ($target:expr, $field:ident, "UPPERCASE") => {{
        $target.$field = Some($crate::RenameRule::UpperCase);
    }};
    ($target:expr, $field:ident, "PascalCase") => {{
        $target.$field = Some($crate::RenameRule::PascalCase);
    }};
    ($target:expr, $field:ident, "camelCase") => {{
        $target.$field = Some($crate::RenameRule::CamelCase);
    }};
    ($target:expr, $field:ident, "snake_case") => {{
        $target.$field = Some($crate::RenameRule::SnakeCase);
    }};
    ($target:expr, $field:ident, "SCREAMING_SNAKE_CASE") => {{
        $target.$field = Some($crate::RenameRule::ScreamingSnakeCase);
    }};
    ($target:expr, $field:ident, "kebab-case") => {{
        $target.$field = Some($crate::RenameRule::KebabCase);
    }};
    ($target:expr, $field:ident, "SCREAMING-KEBAB-CASE") => {{
        $target.$field = Some($crate::RenameRule::ScreamingKebabCase);
    }};
    ($target:expr, $field:ident, $value:tt) => {{
        compile_error!(concat!(
            "unsupported serde casing: `",
            stringify!($value),
            "`"
        ));
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __specta_serde_parse_rename_list {
    ($target:expr; serialize = $value:literal $(, $($rest:tt)*)?) => {
        $crate::__specta_serde_set_str!($target, rename_serialize, $value);
        $( $crate::__specta_serde_parse_rename_list!($target; $($rest)*); )?
    };
    ($target:expr; deserialize = $value:literal $(, $($rest:tt)*)?) => {
        $crate::__specta_serde_set_str!($target, rename_deserialize, $value);
        $( $crate::__specta_serde_parse_rename_list!($target; $($rest)*); )?
    };
    ($target:expr; $unknown:ident $(= $value:expr)? $(, $($rest:tt)*)?) => {
        $( $crate::__specta_serde_parse_rename_list!($target; $($rest)*); )?
    };
    ($target:expr; ) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __specta_serde_parse_rename_all_list {
    ($target:expr; serialize = $value:tt $(, $($rest:tt)*)?) => {
        $crate::__specta_serde_set_case!($target, rename_all_serialize, $value);
        $( $crate::__specta_serde_parse_rename_all_list!($target; $($rest)*); )?
    };
    ($target:expr; deserialize = $value:tt $(, $($rest:tt)*)?) => {
        $crate::__specta_serde_set_case!($target, rename_all_deserialize, $value);
        $( $crate::__specta_serde_parse_rename_all_list!($target; $($rest)*); )?
    };
    ($target:expr; $unknown:ident $(= $value:expr)? $(, $($rest:tt)*)?) => {
        $( $crate::__specta_serde_parse_rename_all_list!($target; $($rest)*); )?
    };
    ($target:expr; ) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __specta_serde_parse_rename_all_fields_list {
    ($target:expr; serialize = $value:tt $(, $($rest:tt)*)?) => {
        $crate::__specta_serde_set_case!($target, rename_all_fields_serialize, $value);
        $( $crate::__specta_serde_parse_rename_all_fields_list!($target; $($rest)*); )?
    };
    ($target:expr; deserialize = $value:tt $(, $($rest:tt)*)?) => {
        $crate::__specta_serde_set_case!($target, rename_all_fields_deserialize, $value);
        $( $crate::__specta_serde_parse_rename_all_fields_list!($target; $($rest)*); )?
    };
    ($target:expr; $unknown:ident $(= $value:expr)? $(, $($rest:tt)*)?) => {
        $( $crate::__specta_serde_parse_rename_all_fields_list!($target; $($rest)*); )?
    };
    ($target:expr; ) => {};
}

/// Parse `#[serde(...)]` attributes into the corresponding serde parser type.
///
/// Supported entrypoints:
/// - `@container` -> [`SerdeContainerAttrs`]
/// - `@variant` -> [`SerdeVariantAttrs`]
/// - `@field` -> [`SerdeFieldAttrs`]
///
/// # Example
///
/// ```rust
/// let parsed = specta_serde::parser!(@container #[serde(
///     rename_all = "camelCase",
///     rename(serialize = "SerName", deserialize = "DeName"),
///     tag = "kind",
///     content = "data"
/// )]);
///
/// assert!(parsed.rename_all_serialize.is_some());
/// assert_eq!(parsed.rename_serialize.as_deref(), Some("SerName"));
/// assert_eq!(parsed.tag.as_deref(), Some("kind"));
/// ```
///
/// ```compile_fail
/// let _ = specta_serde::parser!(@container #[serde(rename_all = "camelCase123")]);
/// ```
#[macro_export]
macro_rules! parser {
    (@container [$($metas:tt)*]) => {{
        let mut parsed = $crate::SerdeContainerAttrs::default();
        $crate::__specta_serde_parse_container_meta_list!(parsed; $($metas)*);
        parsed
    }};

    (@container serde($($items:tt)*)) => {{
        let mut parsed = $crate::SerdeContainerAttrs::default();
        $crate::__specta_serde_parse_container_items!(parsed; $($items)*);
        parsed
    }};
    (@container serde) => {
        $crate::SerdeContainerAttrs::default()
    };
    (@container #[$($meta:tt)*]) => {
        $crate::parser!(@container [$($meta)*])
    };
    (@container $_meta:meta) => {
        $crate::SerdeContainerAttrs::default()
    };
    (@container $($anything:tt)*) => {
        compile_error!("expected `@container #[serde(...)]`")
    };

    (@variant [$($metas:tt)*]) => {{
        let mut parsed = $crate::SerdeVariantAttrs::default();
        $crate::__specta_serde_parse_variant_meta_list!(parsed; $($metas)*);
        parsed
    }};

    (@variant serde($($items:tt)*)) => {{
        let mut parsed = $crate::SerdeVariantAttrs::default();
        $crate::__specta_serde_parse_variant_items!(parsed; $($items)*);
        parsed
    }};
    (@variant serde) => {
        $crate::SerdeVariantAttrs::default()
    };
    (@variant #[$($meta:tt)*]) => {
        $crate::parser!(@variant [$($meta)*])
    };
    (@variant $_meta:meta) => {
        $crate::SerdeVariantAttrs::default()
    };
    (@variant $($anything:tt)*) => {
        compile_error!("expected `@variant #[serde(...)]`")
    };

    (@field [$($metas:tt)*]) => {{
        let mut parsed = $crate::SerdeFieldAttrs::default();
        $crate::__specta_serde_parse_field_meta_list!(parsed; $($metas)*);
        parsed
    }};

    (@field serde($($items:tt)*)) => {{
        let mut parsed = $crate::SerdeFieldAttrs::default();
        $crate::__specta_serde_parse_field_items!(parsed; $($items)*);
        parsed
    }};
    (@field serde) => {
        $crate::SerdeFieldAttrs::default()
    };
    (@field #[$($meta:tt)*]) => {
        $crate::parser!(@field [$($meta)*])
    };
    (@field $_meta:meta) => {
        $crate::SerdeFieldAttrs::default()
    };
    (@field $($anything:tt)*) => {
        compile_error!("expected `@field #[serde(...)]`")
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __specta_serde_parse_container_meta_list {
    ($target:ident; ) => {};
    ($target:ident; , $($rest:tt)*) => {
        $crate::__specta_serde_parse_container_meta_list!($target; $($rest)*);
    };
    ($target:ident; serde($($items:tt)*) , $($rest:tt)*) => {
        $crate::merge_container_attrs(&mut $target, $crate::parser!(@container serde($($items)*)));
        $crate::__specta_serde_parse_container_meta_list!($target; $($rest)*);
    };
    ($target:ident; serde($($items:tt)*)) => {
        $crate::merge_container_attrs(&mut $target, $crate::parser!(@container serde($($items)*)));
    };
    ($target:ident; $unknown:meta , $($rest:tt)*) => {
        $crate::__specta_serde_parse_container_meta_list!($target; $($rest)*);
    };
    ($target:ident; $unknown:meta) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __specta_serde_parse_variant_meta_list {
    ($target:ident; ) => {};
    ($target:ident; , $($rest:tt)*) => {
        $crate::__specta_serde_parse_variant_meta_list!($target; $($rest)*);
    };
    ($target:ident; serde($($items:tt)*) , $($rest:tt)*) => {
        $crate::merge_variant_attrs(&mut $target, $crate::parser!(@variant serde($($items)*)));
        $crate::__specta_serde_parse_variant_meta_list!($target; $($rest)*);
    };
    ($target:ident; serde($($items:tt)*)) => {
        $crate::merge_variant_attrs(&mut $target, $crate::parser!(@variant serde($($items)*)));
    };
    ($target:ident; $unknown:meta , $($rest:tt)*) => {
        $crate::__specta_serde_parse_variant_meta_list!($target; $($rest)*);
    };
    ($target:ident; $unknown:meta) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __specta_serde_parse_field_meta_list {
    ($target:ident; ) => {};
    ($target:ident; , $($rest:tt)*) => {
        $crate::__specta_serde_parse_field_meta_list!($target; $($rest)*);
    };
    ($target:ident; serde($($items:tt)*) , $($rest:tt)*) => {
        $crate::merge_field_attrs(&mut $target, $crate::parser!(@field serde($($items)*)));
        $crate::__specta_serde_parse_field_meta_list!($target; $($rest)*);
    };
    ($target:ident; serde($($items:tt)*)) => {
        $crate::merge_field_attrs(&mut $target, $crate::parser!(@field serde($($items)*)));
    };
    ($target:ident; $unknown:meta , $($rest:tt)*) => {
        $crate::__specta_serde_parse_field_meta_list!($target; $($rest)*);
    };
    ($target:ident; $unknown:meta) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __specta_serde_parse_container_items {
    ($target:ident; ) => {};
    ($target:ident; ,) => {};

    ($target:ident; rename = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, rename_serialize, $value);
        $crate::__specta_serde_set_str!($target, rename_deserialize, $value);
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; rename = $value:literal) => {
        $crate::__specta_serde_set_str!($target, rename_serialize, $value);
        $crate::__specta_serde_set_str!($target, rename_deserialize, $value);
    };
    ($target:ident; rename($($inner:tt)*), $($rest:tt)*) => {
        $crate::__specta_serde_parse_rename_list!($target; $($inner)*);
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; rename($($inner:tt)*)) => {
        $crate::__specta_serde_parse_rename_list!($target; $($inner)*);
    };

    ($target:ident; rename_all = $value:tt, $($rest:tt)*) => {
        $crate::__specta_serde_set_case!($target, rename_all_serialize, $value);
        $crate::__specta_serde_set_case!($target, rename_all_deserialize, $value);
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; rename_all = $value:tt) => {
        $crate::__specta_serde_set_case!($target, rename_all_serialize, $value);
        $crate::__specta_serde_set_case!($target, rename_all_deserialize, $value);
    };
    ($target:ident; rename_all($($inner:tt)*), $($rest:tt)*) => {
        $crate::__specta_serde_parse_rename_all_list!($target; $($inner)*);
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; rename_all($($inner:tt)*)) => {
        $crate::__specta_serde_parse_rename_all_list!($target; $($inner)*);
    };

    ($target:ident; rename_all_fields = $value:tt, $($rest:tt)*) => {
        $crate::__specta_serde_set_case!($target, rename_all_fields_serialize, $value);
        $crate::__specta_serde_set_case!($target, rename_all_fields_deserialize, $value);
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; rename_all_fields = $value:tt) => {
        $crate::__specta_serde_set_case!($target, rename_all_fields_serialize, $value);
        $crate::__specta_serde_set_case!($target, rename_all_fields_deserialize, $value);
    };
    ($target:ident; rename_all_fields($($inner:tt)*), $($rest:tt)*) => {
        $crate::__specta_serde_parse_rename_all_fields_list!($target; $($inner)*);
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; rename_all_fields($($inner:tt)*)) => {
        $crate::__specta_serde_parse_rename_all_fields_list!($target; $($inner)*);
    };

    ($target:ident; deny_unknown_fields, $($rest:tt)*) => {
        $target.deny_unknown_fields = true;
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; deny_unknown_fields) => {
        $target.deny_unknown_fields = true;
    };
    ($target:ident; untagged, $($rest:tt)*) => {
        $target.untagged = true;
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; untagged) => {
        $target.untagged = true;
    };
    ($target:ident; transparent, $($rest:tt)*) => {
        $target.transparent = true;
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; transparent) => {
        $target.transparent = true;
    };
    ($target:ident; variant_identifier, $($rest:tt)*) => {
        $target.variant_identifier = true;
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; variant_identifier) => {
        $target.variant_identifier = true;
    };
    ($target:ident; field_identifier, $($rest:tt)*) => {
        $target.field_identifier = true;
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; field_identifier) => {
        $target.field_identifier = true;
    };

    ($target:ident; tag = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, tag, $value);
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; tag = $value:literal) => {
        $crate::__specta_serde_set_str!($target, tag, $value);
    };
    ($target:ident; content = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, content, $value);
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; content = $value:literal) => {
        $crate::__specta_serde_set_str!($target, content, $value);
    };
    ($target:ident; default = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, default, $value);
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; default = $value:literal) => {
        $crate::__specta_serde_set_str!($target, default, $value);
    };
    ($target:ident; default, $($rest:tt)*) => {
        $target.default = Some(String::from("__default__"));
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; default) => {
        $target.default = Some(String::from("__default__"));
    };
    ($target:ident; remote = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, remote, $value);
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; remote = $value:literal) => {
        $crate::__specta_serde_set_str!($target, remote, $value);
    };
    ($target:ident; from = $value:literal, $($rest:tt)*) => {
        $target.from = Some($crate::ConversionType {
            type_src: String::from($value),
        });
        $target.resolved_from = Some({
            let mut __types = specta::TypeCollection::default();
            <specta::parse_type_from_lit!($value) as specta::Type>::definition(&mut __types)
        });
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; from = $value:literal) => {
        $target.from = Some($crate::ConversionType {
            type_src: String::from($value),
        });
        $target.resolved_from = Some({
            let mut __types = specta::TypeCollection::default();
            <specta::parse_type_from_lit!($value) as specta::Type>::definition(&mut __types)
        });
    };
    ($target:ident; try_from = $value:literal, $($rest:tt)*) => {
        $target.try_from = Some($crate::ConversionType {
            type_src: String::from($value),
        });
        $target.resolved_try_from = Some({
            let mut __types = specta::TypeCollection::default();
            <specta::parse_type_from_lit!($value) as specta::Type>::definition(&mut __types)
        });
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; try_from = $value:literal) => {
        $target.try_from = Some($crate::ConversionType {
            type_src: String::from($value),
        });
        $target.resolved_try_from = Some({
            let mut __types = specta::TypeCollection::default();
            <specta::parse_type_from_lit!($value) as specta::Type>::definition(&mut __types)
        });
    };
    ($target:ident; into = $value:literal, $($rest:tt)*) => {
        $target.into = Some($crate::ConversionType {
            type_src: String::from($value),
        });
        $target.resolved_into = Some({
            let mut __types = specta::TypeCollection::default();
            <specta::parse_type_from_lit!($value) as specta::Type>::definition(&mut __types)
        });
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; into = $value:literal) => {
        $target.into = Some($crate::ConversionType {
            type_src: String::from($value),
        });
        $target.resolved_into = Some({
            let mut __types = specta::TypeCollection::default();
            <specta::parse_type_from_lit!($value) as specta::Type>::definition(&mut __types)
        });
    };
    ($target:ident; crate = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, serde_crate, $value);
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; crate = $value:literal) => {
        $crate::__specta_serde_set_str!($target, serde_crate, $value);
    };
    ($target:ident; expecting = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, expecting, $value);
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; expecting = $value:literal) => {
        $crate::__specta_serde_set_str!($target, expecting, $value);
    };

    ($target:ident; $unknown:ident($($inner:tt)*), $($rest:tt)*) => {
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; $unknown:ident = $value:expr, $($rest:tt)*) => {
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; $unknown:ident, $($rest:tt)*) => {
        $crate::__specta_serde_parse_container_items!($target; $($rest)*);
    };
    ($target:ident; $unknown:ident($($inner:tt)*)) => {};
    ($target:ident; $unknown:ident = $value:expr) => {};
    ($target:ident; $unknown:ident) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __specta_serde_parse_variant_items {
    ($target:ident; ) => {};
    ($target:ident; ,) => {};

    ($target:ident; rename = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, rename_serialize, $value);
        $crate::__specta_serde_set_str!($target, rename_deserialize, $value);
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; rename = $value:literal) => {
        $crate::__specta_serde_set_str!($target, rename_serialize, $value);
        $crate::__specta_serde_set_str!($target, rename_deserialize, $value);
    };
    ($target:ident; rename($($inner:tt)*), $($rest:tt)*) => {
        $crate::__specta_serde_parse_rename_list!($target; $($inner)*);
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; rename($($inner:tt)*)) => {
        $crate::__specta_serde_parse_rename_list!($target; $($inner)*);
    };
    ($target:ident; alias = $value:literal, $($rest:tt)*) => {
        const _: &str = $value;
        $target.aliases.push(String::from($value));
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; alias = $value:literal) => {
        const _: &str = $value;
        $target.aliases.push(String::from($value));
    };
    ($target:ident; rename_all = $value:tt, $($rest:tt)*) => {
        $crate::__specta_serde_set_case!($target, rename_all_serialize, $value);
        $crate::__specta_serde_set_case!($target, rename_all_deserialize, $value);
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; rename_all = $value:tt) => {
        $crate::__specta_serde_set_case!($target, rename_all_serialize, $value);
        $crate::__specta_serde_set_case!($target, rename_all_deserialize, $value);
    };
    ($target:ident; rename_all($($inner:tt)*), $($rest:tt)*) => {
        $crate::__specta_serde_parse_rename_all_list!($target; $($inner)*);
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; rename_all($($inner:tt)*)) => {
        $crate::__specta_serde_parse_rename_all_list!($target; $($inner)*);
    };
    ($target:ident; skip, $($rest:tt)*) => {
        $target.skip_serializing = true;
        $target.skip_deserializing = true;
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; skip) => {
        $target.skip_serializing = true;
        $target.skip_deserializing = true;
    };
    ($target:ident; skip_serializing, $($rest:tt)*) => {
        $target.skip_serializing = true;
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; skip_serializing) => {
        $target.skip_serializing = true;
    };
    ($target:ident; skip_deserializing, $($rest:tt)*) => {
        $target.skip_deserializing = true;
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; skip_deserializing) => {
        $target.skip_deserializing = true;
    };
    ($target:ident; serialize_with = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, serialize_with, $value);
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; serialize_with = $value:literal) => {
        $crate::__specta_serde_set_str!($target, serialize_with, $value);
    };
    ($target:ident; deserialize_with = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, deserialize_with, $value);
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; deserialize_with = $value:literal) => {
        $crate::__specta_serde_set_str!($target, deserialize_with, $value);
    };
    ($target:ident; with = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, with, $value);
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; with = $value:literal) => {
        $crate::__specta_serde_set_str!($target, with, $value);
    };
    ($target:ident; borrow = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, borrow, $value);
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; borrow = $value:literal) => {
        $crate::__specta_serde_set_str!($target, borrow, $value);
    };
    ($target:ident; borrow, $($rest:tt)*) => {
        $target.borrow = Some(String::from("__borrow__"));
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; borrow) => {
        $target.borrow = Some(String::from("__borrow__"));
    };
    ($target:ident; other, $($rest:tt)*) => {
        $target.other = true;
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; other) => {
        $target.other = true;
    };
    ($target:ident; untagged, $($rest:tt)*) => {
        $target.untagged = true;
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; untagged) => {
        $target.untagged = true;
    };

    ($target:ident; $unknown:ident($($inner:tt)*), $($rest:tt)*) => {
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; $unknown:ident = $value:expr, $($rest:tt)*) => {
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; $unknown:ident, $($rest:tt)*) => {
        $crate::__specta_serde_parse_variant_items!($target; $($rest)*);
    };
    ($target:ident; $unknown:ident($($inner:tt)*)) => {};
    ($target:ident; $unknown:ident = $value:expr) => {};
    ($target:ident; $unknown:ident) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __specta_serde_parse_field_items {
    ($target:ident; ) => {};
    ($target:ident; ,) => {};

    ($target:ident; rename = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, rename_serialize, $value);
        $crate::__specta_serde_set_str!($target, rename_deserialize, $value);
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; rename = $value:literal) => {
        $crate::__specta_serde_set_str!($target, rename_serialize, $value);
        $crate::__specta_serde_set_str!($target, rename_deserialize, $value);
    };
    ($target:ident; rename($($inner:tt)*), $($rest:tt)*) => {
        $crate::__specta_serde_parse_rename_list!($target; $($inner)*);
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; rename($($inner:tt)*)) => {
        $crate::__specta_serde_parse_rename_list!($target; $($inner)*);
    };
    ($target:ident; alias = $value:literal, $($rest:tt)*) => {
        const _: &str = $value;
        $target.aliases.push(String::from($value));
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; alias = $value:literal) => {
        const _: &str = $value;
        $target.aliases.push(String::from($value));
    };
    ($target:ident; default = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, default, $value);
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; default = $value:literal) => {
        $crate::__specta_serde_set_str!($target, default, $value);
    };
    ($target:ident; default, $($rest:tt)*) => {
        $target.default = Some(String::from("__default__"));
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; default) => {
        $target.default = Some(String::from("__default__"));
    };
    ($target:ident; flatten, $($rest:tt)*) => {
        $target.flatten = true;
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; flatten) => {
        $target.flatten = true;
    };
    ($target:ident; skip, $($rest:tt)*) => {
        $target.skip_serializing = true;
        $target.skip_deserializing = true;
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; skip) => {
        $target.skip_serializing = true;
        $target.skip_deserializing = true;
    };
    ($target:ident; skip_serializing, $($rest:tt)*) => {
        $target.skip_serializing = true;
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; skip_serializing) => {
        $target.skip_serializing = true;
    };
    ($target:ident; skip_deserializing, $($rest:tt)*) => {
        $target.skip_deserializing = true;
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; skip_deserializing) => {
        $target.skip_deserializing = true;
    };
    ($target:ident; skip_serializing_if = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, skip_serializing_if, $value);
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; skip_serializing_if = $value:literal) => {
        $crate::__specta_serde_set_str!($target, skip_serializing_if, $value);
    };
    ($target:ident; serialize_with = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, serialize_with, $value);
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; serialize_with = $value:literal) => {
        $crate::__specta_serde_set_str!($target, serialize_with, $value);
    };
    ($target:ident; deserialize_with = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, deserialize_with, $value);
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; deserialize_with = $value:literal) => {
        $crate::__specta_serde_set_str!($target, deserialize_with, $value);
    };
    ($target:ident; with = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, with, $value);
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; with = $value:literal) => {
        $crate::__specta_serde_set_str!($target, with, $value);
    };
    ($target:ident; borrow = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, borrow, $value);
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; borrow = $value:literal) => {
        $crate::__specta_serde_set_str!($target, borrow, $value);
    };
    ($target:ident; borrow, $($rest:tt)*) => {
        $target.borrow = Some(String::from("__borrow__"));
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; borrow) => {
        $target.borrow = Some(String::from("__borrow__"));
    };
    ($target:ident; getter = $value:literal, $($rest:tt)*) => {
        $crate::__specta_serde_set_str!($target, getter, $value);
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; getter = $value:literal) => {
        $crate::__specta_serde_set_str!($target, getter, $value);
    };

    ($target:ident; $unknown:ident($($inner:tt)*), $($rest:tt)*) => {
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; $unknown:ident = $value:expr, $($rest:tt)*) => {
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; $unknown:ident, $($rest:tt)*) => {
        $crate::__specta_serde_parse_field_items!($target; $($rest)*);
    };
    ($target:ident; $unknown:ident($($inner:tt)*)) => {};
    ($target:ident; $unknown:ident = $value:expr) => {};
    ($target:ident; $unknown:ident) => {};
}

#[cfg(test)]
mod tests {
    use crate::inflection::RenameRule;

    #[test]
    fn parses_container_attrs() {
        let parsed = crate::parser!(@container #[serde(
            rename(serialize = "s_name", deserialize = "d_name"),
            rename_all = "camelCase",
            rename_all_fields(deserialize = "snake_case"),
            deny_unknown_fields,
            tag = "kind",
            content = "data",
            bound(serialize = "T: Copy", deserialize = "T: Clone"),
            default,
            remote = "crate::Remote",
            transparent,
            from = "String",
            try_from = "String",
            into = "String",
            crate = "serde",
            expecting = "a valid payload",
            variant_identifier,
            field_identifier,
            unknown_container_attr
        )]);

        assert_eq!(parsed.rename_serialize.as_deref(), Some("s_name"));
        assert_eq!(parsed.rename_deserialize.as_deref(), Some("d_name"));
        assert_eq!(parsed.rename_all_serialize, Some(RenameRule::CamelCase));
        assert_eq!(parsed.rename_all_deserialize, Some(RenameRule::CamelCase));
        assert_eq!(
            parsed.rename_all_fields_deserialize,
            Some(RenameRule::SnakeCase)
        );
        assert!(parsed.deny_unknown_fields);
        assert_eq!(parsed.tag.as_deref(), Some("kind"));
        assert_eq!(parsed.content.as_deref(), Some("data"));
        assert_eq!(parsed.default.as_deref(), Some("__default__"));
        assert_eq!(parsed.remote.as_deref(), Some("crate::Remote"));
        assert!(parsed.transparent);
        assert_eq!(
            parsed.from.as_ref().map(|v| v.type_src.as_str()),
            Some("String")
        );
        assert_eq!(
            parsed.try_from.as_ref().map(|v| v.type_src.as_str()),
            Some("String")
        );
        assert_eq!(
            parsed.into.as_ref().map(|v| v.type_src.as_str()),
            Some("String")
        );
        assert_eq!(parsed.serde_crate.as_deref(), Some("serde"));
        assert_eq!(parsed.expecting.as_deref(), Some("a valid payload"));
        assert!(parsed.variant_identifier);
        assert!(parsed.field_identifier);
    }

    #[test]
    fn parses_variant_attrs() {
        let parsed = crate::parser!(@variant #[serde(
            rename = "V",
            alias = "AliasA",
            alias = "AliasB",
            rename_all(deserialize = "UPPERCASE"),
            skip_serializing,
            with = "mod_path",
            bound = "T: Copy",
            borrow = "'a + 'b",
            other,
            untagged,
            unknown_variant_attr
        )]);

        assert_eq!(parsed.rename_serialize.as_deref(), Some("V"));
        assert_eq!(parsed.rename_deserialize.as_deref(), Some("V"));
        assert_eq!(parsed.aliases, vec!["AliasA", "AliasB"]);
        assert_eq!(parsed.rename_all_deserialize, Some(RenameRule::UpperCase));
        assert!(parsed.skip_serializing);
        assert!(!parsed.skip_deserializing);
        assert_eq!(parsed.with.as_deref(), Some("mod_path"));
        assert_eq!(parsed.borrow.as_deref(), Some("'a + 'b"));
        assert!(parsed.other);
        assert!(parsed.untagged);
    }

    #[test]
    fn parses_field_attrs() {
        let parsed = crate::parser!(@field #[serde(
            rename(deserialize = "field_name"),
            alias = "f_alias",
            default = "default_fn",
            flatten,
            skip_serializing_if = "Option::is_none",
            serialize_with = "ser_fn",
            deserialize_with = "de_fn",
            with = "mod_fns",
            borrow,
            bound(deserialize = "T: Clone"),
            getter = "get_field",
            unknown_field_attr
        )]);

        assert_eq!(parsed.rename_deserialize.as_deref(), Some("field_name"));
        assert_eq!(parsed.aliases, vec!["f_alias"]);
        assert_eq!(parsed.default.as_deref(), Some("default_fn"));
        assert!(parsed.flatten);
        assert_eq!(
            parsed.skip_serializing_if.as_deref(),
            Some("Option::is_none")
        );
        assert_eq!(parsed.serialize_with.as_deref(), Some("ser_fn"));
        assert_eq!(parsed.deserialize_with.as_deref(), Some("de_fn"));
        assert_eq!(parsed.with.as_deref(), Some("mod_fns"));
        assert_eq!(parsed.borrow.as_deref(), Some("__borrow__"));
        assert_eq!(parsed.getter.as_deref(), Some("get_field"));
    }

    #[test]
    fn ignores_non_serde_attrs() {
        let c_error = crate::parser!(@container #[error("boom")]);
        let c_doc = crate::parser!(@container #[doc = "hello"]);
        let c_cfg = crate::parser!(@container #[cfg(feature = "my_feature")]);
        let c_bare_serde = crate::parser!(@container #[serde]);
        assert_eq!(c_error, crate::parser::SerdeContainerAttrs::default());
        assert_eq!(c_doc, crate::parser::SerdeContainerAttrs::default());
        assert_eq!(c_cfg, crate::parser::SerdeContainerAttrs::default());
        assert_eq!(c_bare_serde, crate::parser::SerdeContainerAttrs::default());

        let v_error = crate::parser!(@variant #[error("boom")]);
        let v_doc = crate::parser!(@variant #[doc = "hello"]);
        let v_cfg = crate::parser!(@variant #[cfg(feature = "my_feature")]);
        let v_bare_serde = crate::parser!(@variant #[serde]);
        assert_eq!(v_error, crate::parser::SerdeVariantAttrs::default());
        assert_eq!(v_doc, crate::parser::SerdeVariantAttrs::default());
        assert_eq!(v_cfg, crate::parser::SerdeVariantAttrs::default());
        assert_eq!(v_bare_serde, crate::parser::SerdeVariantAttrs::default());

        let f_error = crate::parser!(@field #[error("boom")]);
        let f_doc = crate::parser!(@field #[doc = "hello"]);
        let f_cfg = crate::parser!(@field #[cfg(feature = "my_feature")]);
        let f_bare_serde = crate::parser!(@field #[serde]);
        assert_eq!(f_error, crate::parser::SerdeFieldAttrs::default());
        assert_eq!(f_doc, crate::parser::SerdeFieldAttrs::default());
        assert_eq!(f_cfg, crate::parser::SerdeFieldAttrs::default());
        assert_eq!(f_bare_serde, crate::parser::SerdeFieldAttrs::default());
    }
}
