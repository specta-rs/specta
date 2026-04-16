use std::{borrow::Cow, error, fmt, path::Path, sync::Arc};

use specta::{Types, datatype::DataType};

use crate::{
    Error,
    primitives::{self, GoContext},
};

/// Error type returned by exporter format callbacks.
pub type FormatError = Box<dyn error::Error + Send + Sync + 'static>;

type TypesFormatFn =
    Arc<dyn for<'a> Fn(&'a Types) -> Result<Cow<'a, Types>, FormatError> + Send + Sync>;
type DataTypeFormatFn = Arc<
    dyn for<'a> Fn(&'a Types, &'a DataType) -> Result<Cow<'a, DataType>, FormatError> + Send + Sync,
>;

#[derive(Clone)]
#[doc(hidden)]
pub struct FormatFns {
    pub(crate) types: TypesFormatFn,
    pub(crate) datatype: DataTypeFormatFn,
}

impl<TypesFn, DataTypeFn> From<(TypesFn, DataTypeFn)> for FormatFns
where
    TypesFn: for<'a> Fn(&'a Types) -> Result<Cow<'a, Types>, FormatError> + Send + Sync + 'static,
    DataTypeFn: for<'a> Fn(&'a Types, &'a DataType) -> Result<Cow<'a, DataType>, FormatError>
        + Send
        + Sync
        + 'static,
{
    fn from(format: (TypesFn, DataTypeFn)) -> Self {
        Self {
            types: Arc::new(format.0),
            datatype: Arc::new(format.1),
        }
    }
}

impl fmt::Debug for FormatFns {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FormatFns({:p}, {:p})",
            Arc::as_ptr(&self.types),
            Arc::as_ptr(&self.datatype)
        )
    }
}

/// Allows configuring the format of the final file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Layout {
    /// Flatten all types into a single file. (Idiomatic for Go packages)
    #[default]
    FlatFile,
    /// Produce a dedicated file for each type (Not recommended for Go)
    Files,
}

/// Go language exporter.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Go {
    pub header: Cow<'static, str>,
    pub layout: Layout,
    package_name: String,
    pub(crate) format: Option<FormatFns>,
}

impl Default for Go {
    fn default() -> Self {
        Self {
            header: Cow::Borrowed(""),
            layout: Layout::FlatFile,
            package_name: "bindings".into(),
            format: None,
        }
    }
}

impl Go {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn package_name(mut self, name: impl Into<String>) -> Self {
        self.package_name = name.into();
        self
    }

    pub fn header(mut self, header: impl Into<Cow<'static, str>>) -> Self {
        self.header = header.into();
        self
    }

    pub(crate) fn with_format<TypesFn, DataTypeFn>(mut self, format: (TypesFn, DataTypeFn)) -> Self
    where
        (TypesFn, DataTypeFn): Into<FormatFns>,
    {
        self.format = Some(format.into());
        self
    }

    pub fn export<TypesFn, DataTypeFn>(
        &self,
        types: &Types,
        format: (TypesFn, DataTypeFn),
    ) -> Result<String, Error>
    where
        (TypesFn, DataTypeFn): Into<FormatFns>,
    {
        let mut ctx = GoContext::default();
        let mut body = String::new();

        let exporter = self.clone().with_format(format);
        let formatted_types = exporter.format_types(types)?;
        let types = formatted_types.as_ref();

        for ndt in types.into_sorted_iter() {
            let type_def = primitives::export(&exporter, types, ndt, &mut ctx)?;
            body.push_str(&type_def);
            body.push('\n');
        }

        let mut out = String::new();
        if !exporter.header.is_empty() {
            out.push_str(&exporter.header);
            out.push('\n');
        }

        out.push_str("package ");
        out.push_str(&exporter.package_name);
        out.push_str("\n\n");

        if !ctx.imports.is_empty() {
            out.push_str("import (\n");
            let mut sorted: Vec<_> = ctx.imports.iter().collect();
            sorted.sort();
            for imp in sorted {
                out.push_str(&format!("\t\"{}\"\n", imp));
            }
            out.push_str(")\n\n");
        }

        out.push_str(&body);
        Ok(out)
    }

    pub fn export_to<TypesFn, DataTypeFn>(
        &self,
        path: impl AsRef<Path>,
        types: &Types,
        format: (TypesFn, DataTypeFn),
    ) -> Result<(), Error>
    where
        (TypesFn, DataTypeFn): Into<FormatFns>,
    {
        if self.layout == Layout::Files {
            return Err(Error::UnableToExport(Layout::Files));
        }

        let content = self.export(types, format)?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    pub(crate) fn format_types<'a>(&self, types: &'a Types) -> Result<Cow<'a, Types>, Error> {
        let Some(format) = &self.format else {
            return Ok(Cow::Borrowed(types));
        };

        (format.types)(types).map_err(|err| Error::format("type graph formatter failed", err))
    }
}
