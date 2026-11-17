//! [Kotlin](https://kotlinlang.org) language exporter for [Specta](specta).
//!
//! This crate exports [`specta::Types`] as idiomatic Kotlin declarations. It supports
//! Kotlin collections, nullable types, generics, data classes, enums, and sealed hierarchies.
//!
//! Generated source targets Kotlin 1.9 or newer (`data object` is used for unit declarations).
//! Kotlinx annotations are opt-in through [`Serialization::Kotlinx`] and require the Kotlin
//! serialization compiler plugin plus `kotlinx-serialization-core` and
//! `kotlinx-serialization-json`. Serde's external, internal and adjacent enum representations
//! are reproduced by a generated `KSerializer`. That serializer works on JSON trees, so
//! generated enums only support `kotlinx-serialization-json`; other Kotlinx formats such as
//! CBOR or ProtoBuf are not supported. Shapes that standard Kotlinx serialization would
//! silently encode differently (tuples, untagged enums and generic enums) are rejected
//! instead. Plain declaration export remains available for every shape that can be usefully
//! approximated in Kotlin's nominal type system.
//!
//! # Upgrading from 0.0.6
//!
//! Plain declarations (`Serialization::None`) for Serde's internally and adjacently tagged
//! enums changed to match the wire format:
//!
//! - The lowered tag is no longer a property of each variant class, since the variant itself
//!   identifies it. Construct variants without the tag argument.
//! - An inline struct payload (for example an adjacent `content` value) is now a generated
//!   data class such as `AuthApiErrorSignInRemovedData` instead of `Map<String, Any?>`.
//! - Renamed unit enums record the Serde spelling through a `wireValue` property.
//!
//! `Serialization::Kotlinx` now exports enums instead of rejecting them. Existing Kotlinx
//! output only changes for nullable fields that Serde writes as `null` even when it tolerates
//! their absence, such as `#[serde(default)]` or an optional adjacent payload; those now keep
//! `@EncodeDefault` instead of being omitted.
//!
//! # Usage
//!
//! ```rust
//! use specta::{Type, Types};
//! use specta_kotlin::Kotlin;
//!
//! #[derive(Type)]
//! struct User {
//!     name: String,
//! }
//!
//! let types = Types::default().register::<User>();
//! let source = Kotlin::default()
//!     .export(&types, specta_serde::Format)
//!     .unwrap();
//! # let _ = source;
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod kotlin;
mod render;
mod reserved_names;

pub use error::Error;
pub use kotlin::{IndentStyle, Kotlin, Layout, NamingConvention, Serialization, UnknownType};
