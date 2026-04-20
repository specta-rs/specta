use std::{borrow::Cow, path::Path};

use specta::{Format, Types};

use crate::{Branded, BrandedTypeExporter, Error, Exporter, Layout};

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

    /// Configure the layout of the generated file
    pub fn layout(self, layout: Layout) -> Self {
        Self(self.0.layout(layout))
    }

    /// Configure how `specta_typescript::branded!` types are rendered.
    ///
    /// See [`Exporter::branded_type_impl`] for details.
    pub fn branded_type_impl(
        self,
        builder: impl for<'a> Fn(BrandedTypeExporter<'a>, &Branded) -> Result<Cow<'static, str>, Error>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self(self.0.branded_type_impl(builder))
    }

    /// Export the files into a single string.
    ///
    /// Note: This returns an error if the format is `Format::Files`.
    pub fn export(&self, types: &Types, format: Format) -> Result<String, Error> {
        self.0.export(types, format)
    }

    /// Export the types to a specific file/folder.
    ///
    /// When configured when `format` is `Format::Files`, you must provide a directory path.
    /// Otherwise, you must provide the path of a single file.
    ///
    pub fn export_to(
        &self,
        path: impl AsRef<Path>,
        types: &Types,
        format: Format,
    ) -> Result<(), Error> {
        self.0.export_to(path, types, format)
    }
}

impl AsRef<Exporter> for Typescript {
    fn as_ref(&self) -> &Exporter {
        &self.0
    }
}

impl AsMut<Exporter> for Typescript {
    fn as_mut(&mut self) -> &mut Exporter {
        &mut self.0
    }
}
