use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use specta::{
    datatype::{DataType, Fields, NamedDataType},
    SpectaID, TypeCollection,
};

use crate::{primitives, Error};

/// Allows you to configure how Specta's Typescript exporter will deal with BigInt types ([i64], [i128] etc).
///
/// WARNING: None of these settings affect how your data is actually ser/deserialized.
/// It's up to you to adjust your ser/deserialize settings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BigIntExportBehavior {
    /// Export BigInt as a Typescript `string`
    ///
    /// Doing this in serde is [pretty simple](https://github.com/serde-rs/json/issues/329#issuecomment-305608405).
    String,
    /// Export BigInt as a Typescript `number`.
    ///
    /// WARNING: `JSON.parse` in JS will truncate your number resulting in data loss so ensure your deserializer supports large numbers.
    Number,
    /// Export BigInt as a Typescript `BigInt`.
    ///
    /// You must ensure you deserializer is able to support this.
    BigInt,
    /// Abort the export with an error.
    ///
    /// This is the default behavior because without integration from your serializer and deserializer we can't guarantee data loss won't occur.
    #[default]
    Fail,
}

/// Allows configuring the format of the final types file
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Format {
    /// Produce a Typescript namespace for each Rust module
    Namespaces,
    /// Produce a dedicated file for each Rust module
    Files,
    /// Include the full module path in the types name but keep a flat structure.
    ModulePrefixedName,
    /// Flatten all of the types into a single flat file of types.
    /// This mode doesn't support having multiple types with the same name.
    #[default]
    FlatFile,
}

/// Typescript language exporter.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Typescript {
    pub header: Cow<'static, str>,
    pub framework_header: Cow<'static, str>,
    pub bigint: BigIntExportBehavior,
    pub format: Format,
    pub serde: bool,
    pub(crate) jsdoc: bool,
}

impl Default for Typescript {
    fn default() -> Self {
        Self {
            header: Cow::Borrowed(""),
            framework_header: Cow::Borrowed(
                "// This file has been generated by Specta. DO NOT EDIT.",
            ),
            bigint: Default::default(),
            format: Default::default(),
            serde: false,
            jsdoc: false,
        }
    }
}

impl Typescript {
    /// Construct a new Typescript exporter with the default options configured.
    pub fn new() -> Self {
        Default::default()
    }

    /// Override the header for the exported file.
    /// You should prefer `Self::header` instead unless your a framework.
    #[doc(hidden)] // Although this is hidden it's still public API.
    pub fn framework_header(mut self, header: impl Into<Cow<'static, str>>) -> Self {
        self.framework_header = header.into();
        self
    }

    /// Configure a header for the file.
    ///
    /// This is perfect for configuring lint ignore rules or other file-level comments.
    pub fn header(mut self, header: impl Into<Cow<'static, str>>) -> Self {
        self.header = header.into();
        self
    }

    /// Configure the BigInt handling behaviour
    pub fn bigint(mut self, bigint: BigIntExportBehavior) -> Self {
        self.bigint = bigint;
        self
    }

    /// Configure the format
    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    /// TODO: Explain
    pub fn with_serde(mut self) -> Self {
        self.serde = true;
        self
    }

    /// Export the files into a single string.
    ///
    /// Note: This will return [`Error:UnableToExport`] if the format is `Format::Files`.
    pub fn export(&self, types: &TypeCollection) -> Result<String, Error> {
        if self.serde {
            specta_serde::validate(types)?;
        }

        match self.format {
            Format::Namespaces => {
                let mut out = self.export_internal([].into_iter(), [].into_iter(), types)?;
                let mut module_types: HashMap<_, Vec<_>> = HashMap::new();

                for ndt in types.into_unsorted_iter() {
                    module_types
                        .entry(ndt.module_path().to_string())
                        .or_default()
                        .push(ndt.clone());
                }

                fn export_module(
                    types: &TypeCollection,
                    ts: &Typescript,
                    module_types: &mut HashMap<String, Vec<NamedDataType>>,
                    current_module: &str,
                    indent: usize,
                ) -> Result<String, Error> {
                    let mut out = String::new();
                    if let Some(types_in_module) = module_types.get_mut(current_module) {
                        types_in_module
                            .sort_by(|a, b| a.name().cmp(b.name()).then(a.sid().cmp(&b.sid())));
                        for ndt in types_in_module {
                            out += &"    ".repeat(indent);
                            out += &primitives::export(ts, types, ndt)?;
                            out += "\n\n";
                        }
                    }

                    let mut child_modules = module_types
                        .keys()
                        .filter(|k| {
                            k.starts_with(&format!("{}::", current_module))
                                && k[current_module.len() + 2..].split("::").count() == 1
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    child_modules.sort();

                    for child in child_modules {
                        let module_name = child.split("::").last().unwrap();
                        out += &"    ".repeat(indent);
                        out += &format!("export namespace {module_name} {{\n");
                        out += &export_module(types, ts, module_types, &child, indent + 1)?;
                        out += &"    ".repeat(indent);
                        out += "}\n";
                    }

                    Ok(out)
                }

                let mut root_modules = module_types.keys().cloned().collect::<Vec<_>>();
                root_modules.sort();

                for root_module in root_modules.iter() {
                    out += "import $$specta_ns$$";
                    out += root_module;
                    out += " = ";
                    out += root_module;
                    out += ";\n\n";
                }

                for (i, root_module) in root_modules.iter().enumerate() {
                    if i != 0 {
                        out += "\n";
                    }
                    out += &format!("export namespace {} {{\n", root_module);
                    out += &export_module(types, self, &mut module_types, root_module, 1)?;
                    out += "}";
                }

                Ok(out)
            }
            Format::Files => return Err(Error::UnableToExport),
            Format::FlatFile | Format::ModulePrefixedName => {
                if self.format == Format::FlatFile {
                    let mut map = HashMap::with_capacity(types.len());
                    for dt in types.into_unsorted_iter() {
                        if let Some((existing_sid, existing_impl_location)) =
                            map.insert(dt.name().clone(), (dt.sid(), dt.location()))
                        {
                            if existing_sid != dt.sid() {
                                return Err(Error::DuplicateTypeName {
                                    types: (dt.location(), existing_impl_location),
                                    name: dt.name().clone(),
                                });
                            }
                        }
                    }
                }

                self.export_internal(types.into_sorted_iter(), [].into_iter(), types)
            }
        }
    }

    fn export_internal(
        &self,
        ndts: impl Iterator<Item = NamedDataType>,
        references: impl Iterator<Item = SpectaID>,
        types: &TypeCollection,
    ) -> Result<String, Error> {
        let mut out = self.header.to_string();
        if !out.is_empty() {
            out.push('\n');
        }
        out += &self.framework_header;
        out.push_str("\n");

        for sid in references {
            let ndt = types.get(sid).unwrap();
            out += "import { ";
            out += &ndt.name();
            out += " as ";
            out += &ndt.module_path().replace("::", "_");
            out += "_";
            out += &ndt.name();
            out += " } from \"";
            // TODO: Handle `0` for module path elements
            for i in 1..ndt.module_path().split("::").count() {
                if i == 1 {
                    out += "./";
                } else {
                    out += "../";
                }
            }
            out += &ndt.module_path().replace("::", "/");
            out += "\";\n";
        }

        out.push_str("\n");

        for (i, ndt) in ndts.enumerate() {
            if i != 0 {
                out += "\n\n";
            }

            out += &primitives::export(self, &types, &ndt)?;
        }

        Ok(out)
    }

    /// Export the types to a specific file/folder.
    ///
    /// When configured when `format` is `Format::Files`, you must provide a directory path.
    /// Otherwise, you must provide the path of a single file.
    ///
    pub fn export_to(&self, path: impl AsRef<Path>, types: &TypeCollection) -> Result<(), Error> {
        let path = path.as_ref();

        if self.format == Format::Files {
            if self.serde {
                specta_serde::validate(types)?;
            }

            std::fs::create_dir_all(path)?;

            let mut files = HashMap::<PathBuf, Vec<NamedDataType>>::new();

            for ndt in types.into_sorted_iter() {
                let mut path = PathBuf::from(path);
                for m in ndt.module_path().split("::") {
                    path = path.join(m);
                }
                path.set_extension("ts");
                files.entry(path).or_default().push(ndt);
            }

            let mut used_paths = files.keys().cloned().collect::<HashSet<_>>();

            for (path, ndts) in files {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut references = HashSet::new();
                for ndt in ndts.iter() {
                    crawl_references(ndt.ty(), &mut references);
                }

                std::fs::write(
                    &path,
                    self.export_internal(ndts.into_iter(), references.into_iter(), types)?,
                )?;
            }

            if path.exists() && path.is_dir() {
                fn remove_unused_ts_files(
                    dir: &Path,
                    used_paths: &HashSet<PathBuf>,
                ) -> std::io::Result<()> {
                    for entry in std::fs::read_dir(dir)? {
                        let entry = entry?;
                        let entry_path = entry.path();

                        if entry_path.is_dir() {
                            remove_unused_ts_files(&entry_path, used_paths)?;

                            // Remove empty directories
                            if std::fs::read_dir(&entry_path)?.next().is_none() {
                                std::fs::remove_dir(&entry_path)?;
                            }
                        } else if entry_path.extension().and_then(|ext| ext.to_str()) == Some("ts")
                        {
                            if !used_paths.contains(&entry_path) {
                                std::fs::remove_file(&entry_path)?;
                            }
                        }
                    }
                    Ok(())
                }

                let _ = remove_unused_ts_files(path, &used_paths);
            }
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(
                &path,
                self.export(types).map(|s| format!("{}{s}", self.header))?,
            )?;
        }

        Ok(())
    }
}

fn crawl_references(dt: &DataType, references: &mut HashSet<SpectaID>) {
    match dt {
        DataType::Primitive(..) | DataType::Literal(..) => {}
        DataType::List(list) => {
            crawl_references(list.ty(), references);
        }
        DataType::Map(map) => {
            crawl_references(map.key_ty(), references);
            crawl_references(map.value_ty(), references);
        }
        DataType::Nullable(dt) => {
            crawl_references(dt, references);
        }
        DataType::Struct(s) => {
            crawl_references_fields(&s.fields(), references);
        }
        DataType::Enum(e) => {
            for (_, variant) in e.variants() {
                crawl_references_fields(&variant.fields(), references);
            }
        }
        DataType::Tuple(tuple) => {
            for field in tuple.elements() {
                crawl_references(field, references);
            }
        }
        DataType::Reference(reference) => {
            references.insert(reference.sid());
        }
        DataType::Generic(_) => {}
    }
}

fn crawl_references_fields(fields: &Fields, references: &mut HashSet<SpectaID>) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => {
            for field in fields.fields() {
                if let Some(ty) = field.ty() {
                    crawl_references(ty, references);
                }
            }
        }
        Fields::Named(fields) => {
            for (_, field) in fields.fields() {
                if let Some(ty) = field.ty() {
                    crawl_references(ty, references);
                }
            }
        }
    }
}
