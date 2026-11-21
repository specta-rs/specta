//! Output targets and the [`Exporter`] trait.

use std::{collections::BTreeMap, path::PathBuf};

use specta::{Types, datatype::NamedReference};

use crate::{
    Error, Project,
    elm::{COMMENT_SYMBOL, EXTENSION, PRELUDE},
    module::ModuleImports,
    types::primitive::render_declaration,
};

/// The files an [`Exporter`] produced, keyed by path.
pub type FileRegistry = BTreeMap<PathBuf, String>;

/// Renders a [`Types`] collection into files.
pub trait Exporter {
    /// Render the name used to reference a named type from another type.
    fn format_reference(&self, reference: &NamedReference, types: &Types) -> Result<String, Error>;

    /// Render every type into the files this exporter writes.
    fn render(&self, types: &Types) -> Result<FileRegistry, Error>;

    /// Render every type and write the files into the project.
    fn export(&self, types: &Types) -> Result<(), Error> {
        for (path, content) in self.render(types)? {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|source| Error::create_dir(parent.to_path_buf(), source))?;
            }
            std::fs::write(&path, content)
                .map_err(|source| Error::write_file(path.clone(), source))?;
        }

        Ok(())
    }
}

/// A description of where an [`Exporter`] should write its output.
///
/// Converted into an [`Exporter`] bound to a [`Project`] via [`IntoExporter`].
pub trait IntoExporter {
    /// The exporter this output renders with.
    type Output: Exporter;

    /// Bind this output to `project`.
    fn into_exporter(self, project: &Project) -> Result<Self::Output, Error>;
}

/// Render every type into a single module, such as `Types.elm`.
#[derive(Debug, Clone, Copy)]
pub struct SingleFileOutput<'a>(pub &'a str);

impl IntoExporter for SingleFileOutput<'_> {
    type Output = SingleFileExporter;

    fn into_exporter(self, project: &Project) -> Result<Self::Output, Error> {
        let source_dir = project
            .source_directories()
            .first()
            .ok_or_else(Error::missing_source_directory)?;

        let relative = PathBuf::from(self.0).with_extension("");
        let module = relative
            .components()
            .map(|segment| segment.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join(".");

        if !is_elm_module_name(&module) {
            return Err(Error::invalid_module_name(module));
        }

        Ok(SingleFileExporter {
            target: source_dir.join(&relative).with_extension(EXTENSION),
            module,
        })
    }
}

/// Render every type into `Msg.elm`.
///
/// `Msg` is the module the Elm architecture expects to hold the types which are
/// passed around by an application's `update` function.
#[derive(Debug, Clone, Copy, Default)]
pub struct MsgFileOutput;

impl IntoExporter for MsgFileOutput {
    type Output = SingleFileExporter;

    fn into_exporter(self, project: &Project) -> Result<Self::Output, Error> {
        SingleFileOutput("Msg").into_exporter(project)
    }
}

/// Renders every type into a single Elm module.
#[derive(Debug, Clone)]
pub struct SingleFileExporter {
    target: PathBuf,
    module: String,
}

impl Exporter for SingleFileExporter {
    fn format_reference(&self, reference: &NamedReference, types: &Types) -> Result<String, Error> {
        types
            .get(reference)
            .map(|ndt| ndt.name.to_string())
            .ok_or_else(|| Error::dangling_named_reference("<reference>", format!("{reference:?}")))
    }

    fn render(&self, types: &Types) -> Result<FileRegistry, Error> {
        let mut declarations = String::new();
        let mut imports = ModuleImports::default();

        for ndt in types.iter_sorted().filter(|ndt| ndt.ty.is_some()) {
            render_declaration(&mut declarations, &mut imports, self, types, &ndt.into())?;
        }

        let mut content = format!(
            "module {} exposing (..)\n{COMMENT_SYMBOL}{PRELUDE}\n\n",
            self.module
        );

        let imports = imports.render();
        if !imports.is_empty() {
            content.push_str(&imports);
            content.push_str("\n\n");
        }
        content.push_str(&declarations);

        Ok(FileRegistry::from([(
            self.target.clone(),
            normalise_source(&content),
        )]))
    }
}

/// Trim trailing whitespace and trailing blank lines so output is stable.
fn normalise_source(content: &str) -> String {
    let content = content
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");

    format!("{}\n", content.trim_end())
}

/// Whether `name` is a valid Elm module name, such as `Api.Types`.
fn is_elm_module_name(name: &str) -> bool {
    !name.is_empty()
        && name.split('.').all(|segment| {
            let mut chars = segment.chars();
            chars.next().is_some_and(|c| c.is_ascii_uppercase())
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}
