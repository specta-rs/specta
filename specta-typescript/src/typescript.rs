use std::{borrow::Cow, path::Path};

use specta::TypeCollection;
use specta_serde::SerdeMode;

use crate::{BigIntExportBehavior, Error, Exporter, Layout};

/// JSDoc language exporter.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Typescript(Exporter);

impl Default for Typescript {
    fn default() -> Self {
        Exporter::default().into()
    }
}

impl From<Typescript> for Exporter {
    fn from(value: Typescript) -> Self {
        value.0
    }
}

impl From<Exporter> for Typescript {
    fn from(mut value: Exporter) -> Self {
        value.jsdoc = false;
        Self(value)
    }
}

impl Typescript {
    /// Construct a new JSDoc exporter with the default options configured.
    pub fn new() -> Self {
        Default::default()
    }

    /// Configure a header for the file.
    ///
    /// This is perfect for configuring lint ignore rules or other file-level comments.
    pub fn header(self, header: impl Into<Cow<'static, str>>) -> Self {
        Self(self.0.header(header))
    }

    /// Configure the BigInt handling behaviour
    pub fn bigint(self, bigint: BigIntExportBehavior) -> Self {
        Self(self.0.bigint(bigint))
    }

    /// Configure the layout of the generated file
    pub fn layout(self, layout: Layout) -> Self {
        Self(self.0.layout(layout))
    }

    /// Configure the exporter to use specta-serde with the specified mode
    pub fn with_serde(self, mode: SerdeMode) -> Self {
        Self(self.0.with_serde(mode))
    }

    /// Configure the exporter to export the types for `#[derive(serde::Serialize)]`
    pub fn with_serde_serialize(self) -> Self {
        Self(self.0.with_serde_serialize())
    }

    /// Configure the exporter to export the types for `#[derive(serde::Deserialize)]`
    pub fn with_serde_deserialize(self) -> Self {
        Self(self.0.with_serde_deserialize())
    }

    /// Get a reference to the inner [Exporter] instance.
    pub fn exporter(&self) -> &Exporter {
        &self.0
    }

    /// Export the files into a single string.
    ///
    /// Note: This will return [`Error::UnableToExport`](crate::Error::UnableToExport) if the format is `Format::Files`.
    pub fn export(&self, types: &TypeCollection) -> Result<String, Error> {
        self.0.export(types)
    }

    /// Export the types to a specific file/folder.
    ///
    /// When configured when `format` is `Format::Files`, you must provide a directory path.
    /// Otherwise, you must provide the path of a single file.
    ///
    pub fn export_to(&self, path: impl AsRef<Path>, types: &TypeCollection) -> Result<(), Error> {
        self.0.export_to(path, types)
    }
}

impl AsRef<Exporter> for Typescript {
    fn as_ref(&self) -> &Exporter {
        &self.0
    }
}
