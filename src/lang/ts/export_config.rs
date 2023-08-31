use std::{borrow::Cow, io, path::PathBuf};

use super::{comments, BigIntExportBehavior};

/// The signature for a function responsible for exporting Typescript comments.
pub type CommentFormatterFn = fn(&[Cow<'static, str>]) -> String;

/// The signature for a function responsible for formatter a Typescript file.
pub type FormatterFn = fn(PathBuf) -> io::Result<()>;

/// Options for controlling the behavior of the Typescript exporter.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// How BigInts should be exported.
    pub(crate) bigint: BigIntExportBehavior,
    /// How comments should be rendered.
    pub(crate) comment_exporter: Option<CommentFormatterFn>,
    /// How the resulting file should be formatted.
    pub(crate) formatter: Option<FormatterFn>,
    /// Whether to export types by default.
    /// This can be overridden on a type basis by using `#[specta(export)]`.
    #[cfg(feature = "export")]
    pub(crate) export_by_default: Option<bool>,
}

impl ExportConfig {
    /// Construct a new `ExportConfiguration`
    pub fn new() -> Self {
        Default::default()
    }

    /// Configure the BigInt handling behaviour
    pub fn bigint(mut self, bigint: BigIntExportBehavior) -> Self {
        self.bigint = bigint;
        self
    }

    /// Configure a function which is responsible for styling the comments to be exported
    ///
    /// Implementations:
    ///  - [`js_doc`](crate::lang::ts::js_doc)
    pub fn comment_style(mut self, exporter: CommentFormatterFn) -> Self {
        self.comment_exporter = Some(exporter);
        self
    }

    /// Configure a function which is responsible for formatting the result file or files
    ///
    ///
    /// Implementations:
    ///  - [`prettier`](crate::lang::ts::prettier)
    ///  - [`ESLint`](crate::lang::ts::eslint)
    pub fn formatter(mut self, formatter: FormatterFn) -> Self {
        self.formatter = Some(formatter);
        self
    }

    /// Configure whether or not to export types by default.
    ///
    /// This can be overridden on a specific type by using `#[specta(export)]`.
    ///
    /// This parameter only takes effect when this configuration if passed into [`export::ts_with_cfg`](crate::export::ts_with_cfg)
    #[cfg(feature = "export")]
    pub fn export_by_default(mut self, export: Option<bool>) -> Self {
        self.export_by_default = export;
        self
    }

    /// Run the specified formatter on the given path.
    pub fn run_format(&self, path: PathBuf) -> io::Result<()> {
        if let Some(formatter) = self.formatter {
            formatter(path)?;
        }
        Ok(())
    }
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            bigint: Default::default(),
            comment_exporter: Some(comments::js_doc),
            formatter: None,
            #[cfg(feature = "export")]
            export_by_default: None,
        }
    }
}
