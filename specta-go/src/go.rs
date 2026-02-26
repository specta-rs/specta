use std::{borrow::Cow, path::Path};

use specta::TypeCollection;
use specta_serde::SerdeMode;

use crate::{
    primitives::{self, GoContext},
    Error,
};

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
    pub serde: Option<SerdeMode>,
}

impl Default for Go {
    fn default() -> Self {
        Self {
            header: Cow::Borrowed(""),
            layout: Layout::FlatFile,
            package_name: "bindings".into(),
            serde: Some(SerdeMode::Both),
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

    pub fn with_serde(mut self, mode: SerdeMode) -> Self {
        self.serde = Some(mode);
        self
    }

    pub fn export(&self, types: &TypeCollection) -> Result<String, Error> {
        let mut ctx = GoContext::default();
        let mut body = String::new();

        let types = if let Some(mode) = self.serde {
            let mut types = types.clone();
            specta_serde::apply(&mut types, mode)?;
            Cow::Owned(types)
        } else {
            Cow::Borrowed(types)
        };

        for ndt in types.into_sorted_iter() {
            let type_def = primitives::export(&self, &types, ndt, &mut ctx)?;
            body.push_str(&type_def);
            body.push('\n');
        }

        let mut out = String::new();
        if !self.header.is_empty() {
            out.push_str(&self.header);
            out.push('\n');
        }

        out.push_str("package ");
        out.push_str(&self.package_name);
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

    pub fn export_to(&self, path: impl AsRef<Path>, types: &TypeCollection) -> Result<(), Error> {
        if self.layout == Layout::Files {
            return Err(Error::UnableToExport(Layout::Files));
        }

        let content = self.export(types)?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}
