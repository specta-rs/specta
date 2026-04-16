use std::{borrow::Cow, path::Path};

use specta::{
    Types,
    datatype::DataType,
};

use crate::{Branded, BrandedTypeExporter, Error, Exporter, FormatError, Layout};

/// JSDoc language exporter.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct JSDoc(Exporter);

impl Default for JSDoc {
    fn default() -> Self {
        let mut exporter = Exporter::default();
        exporter.jsdoc = true;
        exporter.into()
    }
}

impl From<JSDoc> for Exporter {
    fn from(value: JSDoc) -> Self {
        value.0
    }
}

impl From<Exporter> for JSDoc {
    fn from(mut value: Exporter) -> Self {
        value.jsdoc = true;
        Self(value)
    }
}

impl JSDoc {
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

    /// Add some custom Typescript or Javascript code that is exported as part of the bindings.
    pub fn framework_runtime(
        self,
        builder: impl Fn(crate::FrameworkExporter) -> Result<Cow<'static, str>, Error>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self(self.0.framework_runtime(builder))
    }

    /// Export the files into a single string.
    ///
    /// Note: This returns an error if the format is `Format::Files`.
    pub fn export<TypesFn, DataTypeFn>(
        &self,
        types: &Types,
        format: (TypesFn, DataTypeFn),
    ) -> Result<String, Error>
    where
        TypesFn: for<'a> Fn(&'a Types) -> Result<Cow<'a, Types>, FormatError>
            + Send
            + Sync
            + 'static,
        DataTypeFn: for<'a> Fn(&'a Types, &'a DataType) -> Result<Cow<'a, DataType>, FormatError>
            + Send
            + Sync
            + 'static,
    {
        self.0.export(types, format)
    }

    /// Export the types to a specific file/folder.
    ///
    /// When configured when `format` is `Format::Files`, you must provide a directory path.
    /// Otherwise, you must provide the path of a single file.
    ///
    pub fn export_to<TypesFn, DataTypeFn>(
        &self,
        path: impl AsRef<Path>,
        types: &Types,
        format: (TypesFn, DataTypeFn),
    ) -> Result<(), Error>
    where
        TypesFn: for<'a> Fn(&'a Types) -> Result<Cow<'a, Types>, FormatError>
            + Send
            + Sync
            + 'static,
        DataTypeFn: for<'a> Fn(&'a Types, &'a DataType) -> Result<Cow<'a, DataType>, FormatError>
            + Send
            + Sync
            + 'static,
    {
        self.0.export_to(path, types, format)
    }
}

impl AsRef<Exporter> for JSDoc {
    fn as_ref(&self) -> &Exporter {
        &self.0
    }
}

impl AsMut<Exporter> for JSDoc {
    fn as_mut(&mut self) -> &mut Exporter {
        &mut self.0
    }
}
