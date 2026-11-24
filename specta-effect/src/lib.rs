//! [Effect Schema](https://effect.website/docs/schema/introduction/) exporter for [Specta](specta).
//!
//! This exporter emits Effect Schema values only. It does not emit a second
//! set of TypeScript type declarations or JSDoc bindings.
//!
//! # Usage
//!
//! ```rust,no_run
//! use specta::{Type, Types};
//! use specta_effect::Effect;
//!
//! #[derive(Type)]
//! pub struct User {
//!     pub id: u32,
//!     pub name: String,
//! }
//!
//! let types = Types::default().register::<User>();
//! Effect::default()
//!     .export_to("./schemas.ts", &types, specta_serde::Format)
//!     .unwrap();
//! ```
//!
//! The generated file imports Schema from `effect` and exports a schema for
//! every registered Rust type. Generic Rust types become schema factories, so a
//! reference such as `Wrapper<User>` is emitted as `WrapperSchema(UserSchema)`.
//! Generic map keys receive an additional `$key$T` schema parameter because
//! Effect records validate encoded property keys.
//!
//! This exporter targets Effect 4.1. Recursive references use
//! `Schema.suspend`; because no companion TypeScript declarations are emitted,
//! the recursive edge is typed as `Schema.Codec<unknown>`. Serde field alias
//! groups are expanded into schema unions, up to 256 combinations. Wide integer
//! primitives are rejected by default to avoid silent JSON precision loss; use
//! [`define`] or a [`Format`](specta::Format) to remap them explicitly.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod effect;
mod error;
mod map_keys;
mod opaque;
pub mod primitives;
mod references;
mod reserved_names;
mod types;

pub use effect::{Effect, FrameworkExporter, Layout};
pub use error::Error;
pub use opaque::define;
pub use references::collect_references;
pub use types::{Any, BigInt, Never, Number, Unknown};
