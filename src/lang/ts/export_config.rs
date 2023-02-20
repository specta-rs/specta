use super::{comments, BigIntExportBehavior, CommentFormatterFn};

/// allows you to control the behavior of the Typescript exporter
pub struct ExportConfiguration {
    /// control the bigint exporting behavior
    pub(crate) bigint: BigIntExportBehavior,
    /// control the style of exported comments
    pub(crate) comment_exporter: Option<CommentFormatterFn>,
    /// Configure whether or not to export types by default.
    /// This can be overridden on a type basis by using `#[specta(export)]`
    #[cfg(feature = "export")]
    pub(crate) export_by_default: Option<bool>,
}

impl ExportConfiguration {
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
    pub fn comment_style(mut self, exporter: Option<CommentFormatterFn>) -> Self {
        self.comment_exporter = exporter;
        self
    }

    /// Configure whether or not to export types by default.
    /// Note: This parameter only work if this configuration if passed into [crate::export::ts]
    #[cfg(feature = "export")]
    pub fn export_by_default(mut self, x: Option<bool>) -> Self {
        self.export_by_default = x;
        self
    }
}

impl Default for ExportConfiguration {
    fn default() -> Self {
        Self {
            bigint: Default::default(),
            comment_exporter: Some(comments::js_doc),
            #[cfg(feature = "export")]
            export_by_default: None,
        }
    }
}
