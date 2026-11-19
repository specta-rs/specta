//! Elm `import` declarations which generated modules need.

use std::collections::BTreeSet;

use crate::elm::ElmCoreLibImport;

/// The core modules required by a generated module.
///
/// Every named type is rendered into the same file, so the only imports needed
/// are Elm's own core modules. A multi-file exporter would also track imports of
/// other generated modules here.
#[derive(Debug, Clone, Default)]
pub(crate) struct ModuleImports {
    core: BTreeSet<ElmCoreLibImport>,
}

impl ModuleImports {
    /// Record a required core module.
    pub(crate) fn require_core(&mut self, import: ElmCoreLibImport) {
        self.core.insert(import);
    }

    /// Render the declarations, sorted so that the output is deterministic.
    pub(crate) fn render(&self) -> String {
        self.core
            .iter()
            .map(|import| format!("import {import} exposing ({import})"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
