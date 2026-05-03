//! Easily export your Rust types to other languages
//!
//! This crate contains the macro which are reexported by the `specta` crate.
//! You shouldn't need to use this crate directly.
//! Checkout [Specta](https://docs.rs/specta).
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

#[cfg(feature = "DO_NOT_USE_function")]
mod specta;
mod r#type;
mod utils;

use quote::quote;
use syn::{Error, LitStr, Type, parse_macro_input};

/// Implements `specta::Type` for a given struct or enum.
///
/// # Attributes
///
/// `Type` supports `#[specta(...)]` attributes on containers, variants, and fields.
/// It also understands selected Rust and Serde attributes.
///
/// ## `#[specta(...)]` container attributes
///
/// These can be used on the struct or enum deriving `Type`.
///
/// - `#[specta(type = T)]` overrides the generated type definition with `T`.
///   `T` may be a path or Rust type, such as `String`, `Vec<T>`, or `(String, i32)`.
/// - `#[specta(crate = path::to::specta)]` uses a custom path to the `specta` crate.
/// - `#[specta(inline)]` or `#[specta(inline = true)]` inlines this type where it is referenced.
///   Use `#[specta(inline = false)]` to disable it.
/// - `#[specta(remote = path::ToType)]` implements `Type` for a remote type instead of the
///   local derive input name.
/// - `#[specta(collect)]` or `#[specta(collect = true)]` enables collection for this type when
///   the `collect` feature is used. Use `#[specta(collect = false)]` to prevent collection.
/// - `#[specta(skip_attr = "attr_name")]` ignores attributes named `attr_name` while parsing and
///   while collecting runtime attributes. This may be repeated.
/// - `#[specta(transparent)]` or `#[specta(transparent = true)]` treats a struct as its single
///   non-skipped field. Use `#[specta(transparent = false)]` to disable it.
/// - `#[specta(bound = "T: Type")]` replaces the automatically inferred `Type` bounds.
///   Use `#[specta(bound = "")]` to emit no inferred bounds.
///
/// `#[specta(type = ...)]` cannot be combined with `#[specta(transparent)]`.
/// `#[specta(transparent)]` is only valid on structs with exactly one non-skipped field.
///
/// ## `#[specta(...)]` variant attributes
///
/// These can be used on enum variants.
///
/// - `#[specta(type = T)]` or `#[specta(r#type = T)]` overrides the generated variant payload
///   type with `T`.
/// - `#[specta(skip)]` or `#[specta(skip = true)]` marks the variant as skipped.
///   Use `#[specta(skip = false)]` to disable it.
/// - `#[specta(inline)]` or `#[specta(inline = true)]` inlines the first unnamed field of the
///   variant. Use `#[specta(inline = false)]` to disable it.
///
/// ## `#[specta(...)]` field attributes
///
/// These can be used on struct fields and enum variant fields.
///
/// - `#[specta(type = T)]` overrides the generated field type with `T`.
/// - `#[specta(inline)]` or `#[specta(inline = true)]` inlines the field type.
///   Use `#[specta(inline = false)]` to disable it.
/// - `#[specta(skip)]` or `#[specta(skip = true)]` skips generating the field type.
///   Use `#[specta(skip = false)]` to disable it.
/// - `#[specta(optional)]` or `#[specta(optional = true)]` marks the field as optional.
///   This is commonly used with `Option<T>` to export `{ a?: T | null }` instead of
///   `{ a: T | null }`.
/// - `#[specta(default)]` or `#[specta(default = true)]` is an alias for `optional`.
///
/// ## Rust attributes
///
/// These are read on containers, variants, and fields.
///
/// - `#[doc = "..."]` is exported as documentation.
/// - `#[deprecated]` marks the item as deprecated.
/// - `#[deprecated = "..."]` marks the item as deprecated with a note.
/// - `#[deprecated(note = "...")]` marks the item as deprecated with a note.
/// - `#[deprecated(since = "...", note = "...")]` marks the item as deprecated with a version
///   and note.
///
/// `#[repr(transparent)]` is also accepted on containers as an alias for
/// `#[specta(transparent)]`.
///
/// ## Serde attributes
///
/// Specta can read selected `#[serde(...)]` attributes. Prefer Serde attributes when the same
/// behavior should apply to Serde and Specta.
///
/// These are always understood by the derive macro:
///
/// - Container: `#[serde(transparent)]` acts like `#[specta(transparent)]`.
/// - Field: `#[serde(skip)]` acts like `#[specta(skip)]`.
/// - Variant: `#[serde(skip)]` acts like `#[specta(skip)]`.
///
/// With the `serde` feature enabled, the following Serde attributes are also preserved as
/// runtime attributes.
/// To apply these attributes during export, use
/// [`specta_serde::Format`](https://docs.rs/specta-serde/latest/specta_serde/struct.Format.html)
/// or
/// [`specta_serde::FormatPhases`](https://docs.rs/specta-serde/latest/specta_serde/struct.FormatPhases.html).
/// See the [`specta-serde` docs](https://docs.rs/specta-serde) for details.
///
/// Container attributes:
///
/// - `#[serde(rename = "...")]`
/// - `#[serde(rename(serialize = "..."))]`
/// - `#[serde(rename(deserialize = "..."))]`
/// - `#[serde(rename_all = "...")]`
/// - `#[serde(rename_all(serialize = "..."))]`
/// - `#[serde(rename_all(deserialize = "..."))]`
/// - `#[serde(rename_all_fields = "...")]`
/// - `#[serde(rename_all_fields(serialize = "..."))]`
/// - `#[serde(rename_all_fields(deserialize = "..."))]`
/// - `#[serde(tag = "...")]`
/// - `#[serde(content = "...")]`
/// - `#[serde(untagged)]`
/// - `#[serde(default)]` or `#[serde(default = "...")]`
/// - `#[serde(transparent)]`
/// - `#[serde(from = "T")]`
/// - `#[serde(try_from = "T")]`
/// - `#[serde(into = "T")]`
/// - `#[serde(variant_identifier)]`
/// - `#[serde(field_identifier)]`
///
/// Variant attributes:
///
/// - `#[serde(rename = "...")]`
/// - `#[serde(rename(serialize = "..."))]`
/// - `#[serde(rename(deserialize = "..."))]`
/// - `#[serde(alias = "...")]`
/// - `#[serde(rename_all = "...")]`
/// - `#[serde(rename_all(serialize = "..."))]`
/// - `#[serde(rename_all(deserialize = "..."))]`
/// - `#[serde(skip)]`
/// - `#[serde(skip_serializing)]`
/// - `#[serde(skip_deserializing)]`
/// - `#[serde(serialize_with = "...")]`
/// - `#[serde(deserialize_with = "...")]`
/// - `#[serde(with = "...")]`
/// - `#[serde(other)]`
/// - `#[serde(untagged)]`
///
/// Field attributes:
///
/// - `#[serde(rename = "...")]`
/// - `#[serde(rename(serialize = "..."))]`
/// - `#[serde(rename(deserialize = "..."))]`
/// - `#[serde(alias = "...")]`
/// - `#[serde(default)]` or `#[serde(default = "...")]`
/// - `#[serde(flatten)]`
/// - `#[serde(skip)]`
/// - `#[serde(skip_serializing)]`
/// - `#[serde(skip_deserializing)]`
/// - `#[serde(skip_serializing_if = "...")]`
/// - `#[serde(serialize_with = "...")]`
/// - `#[serde(deserialize_with = "...")]`
/// - `#[serde(with = "...")]`
///
/// Supported rename casing values are:
///
/// - `"lowercase"`
/// - `"UPPERCASE"`
/// - `"PascalCase"`
/// - `"camelCase"`
/// - `"snake_case"`
/// - `"SCREAMING_SNAKE_CASE"`
/// - `"kebab-case"`
/// - `"SCREAMING-KEBAB-CASE"`
///
/// ## Example
///
/// ```ignore
/// use specta::Type;
///
/// // Use it on structs
/// #[derive(Type)]
/// pub struct MyCustomStruct {
///     pub name: String,
/// }
///
/// #[derive(Type)]
/// pub struct MyCustomStruct2(String, i32, bool);
///
/// // Use it on enums
/// #[derive(Type)]
/// pub enum MyCustomType {
///     VariantOne,
///     VariantTwo(String, i32),
///     VariantThree { name: String, age: i32 },
/// }
/// ```
///
/// ## Known limitations
///
///  - Const generics will not be exported within user-defined types which define const generics
///  - Associated constants or types can't be used
///
#[proc_macro_derive(Type, attributes(specta))]
pub fn derive_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    r#type::derive(input).unwrap_or_else(|err| err.into_compile_error().into())
}

/// Parses a string literal into a Rust type token stream.
///
/// This is an internal helper proc macro used by Specta macros to turn a
/// literal like `"Option<String>"` into a Rust type at compile time.
#[proc_macro]
pub fn parse_type_from_lit(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let lit = parse_macro_input!(input as LitStr);

    match syn::parse_str::<Type>(&lit.value()) {
        Ok(ty) => quote!(#ty).into(),
        Err(err) => Error::new_spanned(lit, format!("invalid type literal: {err}"))
            .to_compile_error()
            .into(),
    }
}

/// Prepares a function to have its types extracted using `specta::function::fn_datatype!`
///
/// ## Example
///
/// ```ignore
/// #[specta::specta]
/// fn my_function(arg1: i32, arg2: bool) -> &'static str {
///     "Hello World"
/// }
/// ```
#[proc_macro_attribute]
#[cfg(feature = "DO_NOT_USE_function")]
pub fn specta(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    specta::attribute(item).unwrap_or_else(|err| err.into_compile_error().into())
}
