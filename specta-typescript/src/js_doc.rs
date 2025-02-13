use std::{borrow::Cow, path::Path};

use specta::TypeCollection;

use crate::{BigIntExportBehavior, Error, Typescript};

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

impl JSDoc {
    /// Construct a new JSDoc exporter with the default options configured.
    pub fn new() -> Self {
        Default::default()
    }

    /// Override the header for the exported file.
    /// You should prefer `Self::header` instead unless your a framework.
    #[doc(hidden)] // Although this is hidden it's still public API.
    pub fn framework_header(mut self, header: impl Into<Cow<'static, str>>) -> Self {
        self.0.framework_header = header.into();
        self
    }

    /// Configure a header for the file.
    ///
    /// This is perfect for configuring lint ignore rules or other file-level comments.
    pub fn header(mut self, header: impl Into<Cow<'static, str>>) -> Self {
        self.0.header = header.into();
        self
    }

    /// Configure the BigInt handling behaviour
    pub fn bigint(mut self, bigint: BigIntExportBehavior) -> Self {
        self.0.bigint = bigint;
        self
    }

    /// TODO
    pub fn export(&self, types: &TypeCollection) -> Result<String, Error> {
        self.0.export(types)
    }

    /// TODO
    pub fn export_to(&self, path: impl AsRef<Path>, types: &TypeCollection) -> Result<(), Error> {
        self.0.export_to(path, types)
    }
}
