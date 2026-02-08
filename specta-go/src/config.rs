use std::{borrow::Cow, fmt, io, path::PathBuf};

use specta_serde::SerdeMode;

/// Allows you to configure how the exporter will deal with BigInt types ([i64], [i128] etc).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BigIntExportBehavior {
    /// Export BigInt as a Go `string`
    String,
    /// Export BigInt as a Go `int64` / `uint64`.
    Number,
    /// Same as Number for Go (Go handles int64 natively).
    BigInt,
    /// Abort the export with an error.
    #[default]
    Fail,
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

#[derive(Debug, Clone)]
pub struct Exporter {
    pub header: Cow<'static, str>,
    pub bigint: BigIntExportBehavior,
    pub layout: Layout,
    pub serde: Option<SerdeMode>,
}

impl Default for Exporter {
    fn default() -> Self {
        Self {
            header: Cow::Borrowed(""),
            bigint: Default::default(),
            layout: Default::default(),
            serde: Some(SerdeMode::Both),
        }
    }
}

impl Exporter {
    pub fn header(mut self, header: impl Into<Cow<'static, str>>) -> Self {
        self.header = header.into();
        self
    }

    pub fn bigint(mut self, bigint: BigIntExportBehavior) -> Self {
        self.bigint = bigint;
        self
    }

    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_serde(mut self, mode: SerdeMode) -> Self {
        self.serde = Some(mode);
        self
    }

    pub fn with_serde_serialize(self) -> Self {
        self.with_serde(SerdeMode::Serialize)
    }

    pub fn with_serde_deserialize(self) -> Self {
        self.with_serde(SerdeMode::Deserialize)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Fmt error: {0}")]
    Fmt(#[from] fmt::Error),
    #[error("Serde error: {0}")]
    Serde(#[from] specta_serde::Error),
    #[error("Forbidden name: {name} in {path}")]
    ForbiddenName { path: String, name: String },
    #[error("BigInt forbidden in {path}")]
    BigIntForbidden { path: String },
    #[error("Unable to export layout: {0:?}")]
    UnableToExport(Layout),
}
