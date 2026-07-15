//! [C#](https://learn.microsoft.com/dotnet/csharp/) language exporter for [Specta](specta).
//!
//! The exporter generates nullable-aware C# records, enums, generic types, XML documentation,
//! and `System.Text.Json` property-name metadata. Data-carrying Rust enums are emitted as class
//! hierarchies; applications must provide a converter appropriate to their chosen wire format.
//! It supports flat, namespace, module-prefixed, and per-type file layouts.
//! Generated bindings target C# 11 or newer because required members are used for non-optional
//! fields.
//!
//! ```rust
//! use specta::Types;
//! use specta_csharp::CSharp;
//!
//! #[derive(specta::Type)]
//! struct User {
//!     name: String,
//! }
//!
//! let types = Types::default().register::<User>();
//! let bindings = CSharp::default().export(&types, specta_serde::Format).unwrap();
//! assert!(bindings.contains("public sealed record User"));
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
pub mod primitives;
mod render;

pub use error::Error;

use std::{borrow::Cow, collections::BTreeMap, fmt, path::Path};

use specta::{Format, Types};

/// Layout of generated C# declarations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Layout {
    /// Put every type in one namespace and one file.
    #[default]
    FlatFile,
    /// Put Rust modules into nested C# namespaces in one file.
    Namespaces,
    /// Prefix flattened type names with their Rust module path.
    ModulePrefixedName,
    /// Write one `.cs` file per type, in directories matching Rust modules.
    Files,
}

impl fmt::Display for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Visibility applied to generated top-level types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Visibility {
    /// Generate public types.
    #[default]
    Public,
    /// Generate assembly-internal types.
    Internal,
}

impl Visibility {
    pub(crate) const fn keyword(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Internal => "internal",
        }
    }
}

/// C# language exporter.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CSharp {
    /// Content written before generated declarations.
    pub header: Cow<'static, str>,
    /// Root C# namespace. Set to an empty string to omit a namespace.
    pub namespace: Cow<'static, str>,
    /// Output layout.
    pub layout: Layout,
    /// Top-level type visibility.
    pub visibility: Visibility,
    /// One indentation unit.
    pub indent: Cow<'static, str>,
    raw: Vec<Cow<'static, str>>,
    pub(crate) opaque_types: BTreeMap<Cow<'static, str>, Cow<'static, str>>,
}

impl Default for CSharp {
    fn default() -> Self {
        Self {
            header: Cow::Borrowed(""),
            namespace: Cow::Borrowed("Specta.Generated"),
            layout: Layout::FlatFile,
            visibility: Visibility::Public,
            indent: Cow::Borrowed("    "),
            raw: Vec::new(),
            opaque_types: BTreeMap::new(),
        }
    }
}

impl CSharp {
    /// Construct an exporter with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set content written before generated declarations.
    pub fn header(mut self, header: impl Into<Cow<'static, str>>) -> Self {
        self.header = header.into();
        self
    }

    /// Set the root namespace. An empty namespace emits declarations at global scope.
    pub fn namespace(mut self, namespace: impl Into<Cow<'static, str>>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Set the output layout.
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    /// Set generated top-level visibility.
    pub fn visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Set one indentation unit, such as four spaces or a tab.
    pub fn indent(mut self, indent: impl Into<Cow<'static, str>>) -> Self {
        self.indent = indent.into();
        self
    }

    /// Append raw C# after generated declarations.
    pub fn with_raw(mut self, raw: impl Into<Cow<'static, str>>) -> Self {
        self.raw.push(raw.into());
        self
    }

    /// Configure the C# use-site type for an exporter-specific opaque Rust reference.
    pub fn opaque_type(
        mut self,
        rust_type_name: impl Into<Cow<'static, str>>,
        csharp_type: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.opaque_types
            .insert(rust_type_name.into(), csharp_type.into());
        self
    }

    /// Export all supplied types into a single C# source string.
    pub fn export(&self, types: &Types, format: impl Format) -> Result<String, Error> {
        if self.layout == Layout::Files {
            return Err(Error::ExportRequiresExportTo(self.layout));
        }
        render::export(self, types, &format)
    }

    /// Export bindings to a file, or to a directory for [`Layout::Files`].
    pub fn export_to(
        &self,
        path: impl AsRef<Path>,
        types: &Types,
        format: impl Format,
    ) -> Result<(), Error> {
        render::export_to(self, path.as_ref(), types, &format)
    }
}
