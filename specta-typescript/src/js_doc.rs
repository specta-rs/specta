use std::{borrow::Cow, path::Path};

use specta::TypeCollection;

use crate::{BigIntExportBehavior, Error, Format, Typescript};

/// JSDoc language exporter.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct JSDoc(pub Typescript);

impl Default for JSDoc {
    fn default() -> Self {
        Typescript::default().into()
    }
}

impl From<Typescript> for JSDoc {
    fn from(mut ts: Typescript) -> Self {
        ts.jsdoc = true;
        Self(ts)
    }
}

impl From<JSDoc> for Typescript {
    fn from(mut jsdoc: JSDoc) -> Self {
        jsdoc.0.jsdoc = false;
        jsdoc.0
    }
}

impl JSDoc {
    /// Construct a new JSDoc exporter with the default options configured.
    pub fn new() -> Self {
        Default::default()
    }

    /// Override the header for the exported file.
    /// You should prefer `Self::header` instead unless your a framework.
    #[doc(hidden)] // Although this is hidden it's still public API.
    pub fn framework_header(self, header: impl Into<Cow<'static, str>>) -> Self {
        Self(self.0.framework_header(header))
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

    /// Configure the format
    pub fn format(self, format: Format) -> Self {
        Self(self.0.format(format))
    }

    /// TODO: Explain
    pub fn with_serde(self) -> Self {
        Self(self.0.with_serde())
    }

    /// Export the files into a single string.
    ///
    /// Note: This will return [`Error:UnableToExport`] if the format is `Format::Files`.
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
