use std::{borrow::Cow, fmt, path::Path, sync::Arc};

use specta::{
    Format, Types,
    datatype::{DataType, Fields, Reference},
};

use crate::{
    Error,
    primitives::{self, GoContext},
};

type TypesFormatFn =
    Arc<dyn for<'a> Fn(&'a Types) -> Result<Cow<'a, Types>, specta::FormatError> + Send + Sync>;
type DataTypeFormatFn = Arc<
    dyn for<'a> Fn(&'a Types, &'a DataType) -> Result<Cow<'a, DataType>, specta::FormatError>
        + Send
        + Sync,
>;

#[derive(Clone)]
pub(crate) struct FormatFns {
    pub(crate) types: TypesFormatFn,
    pub(crate) datatype: DataTypeFormatFn,
}

impl From<Format> for FormatFns {
    fn from(format: Format) -> Self {
        Self {
            types: Arc::new(format.format_types),
            datatype: Arc::new(format.format_dt),
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

    pub(crate) fn with_format(mut self, format: Format) -> Self {
        self.format = Some(format.into());
        self
    }

    pub fn export(&self, types: &Types, format: Format) -> Result<String, Error> {
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

    pub fn export_to(
        &self,
        path: impl AsRef<Path>,
        types: &Types,
        format: Format,
    ) -> Result<(), Error> {
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

        let mapped_types = (format.types)(types)
            .map_err(|err| Error::format("type graph formatter failed", err))?;
        Ok(Cow::Owned(
            map_types_for_datatype_format(self, mapped_types.as_ref())?.into_owned(),
        ))
    }
}

fn map_datatype_format(exporter: &Go, types: &Types, dt: &DataType) -> Result<DataType, Error> {
    let Some(format) = exporter.format.as_ref() else {
        return Ok(dt.clone());
    };

    let mapped = (format.datatype)(types, dt)
        .map_err(|err| Error::format("datatype formatter failed", err))?;

    match mapped {
        Cow::Borrowed(dt) => map_datatype_format_children(exporter, types, dt.clone()),
        Cow::Owned(dt) => map_datatype_format_children(exporter, types, dt),
    }
}

fn map_datatype_format_children(
    exporter: &Go,
    types: &Types,
    mut dt: DataType,
) -> Result<DataType, Error> {
    match &mut dt {
        DataType::Primitive(_) => {}
        DataType::List(list) => {
            list.ty = Box::new(map_datatype_format(exporter, types, &list.ty)?);
        }
        DataType::Map(map) => {
            let key = map_datatype_format(exporter, types, map.key_ty())?;
            let value = map_datatype_format(exporter, types, map.value_ty())?;
            map.set_key_ty(key);
            map.set_value_ty(value);
        }
        DataType::Nullable(inner) => {
            **inner = map_datatype_format(exporter, types, inner)?;
        }
        DataType::Struct(strct) => map_datatype_fields(exporter, types, &mut strct.fields)?,
        DataType::Enum(enm) => {
            for (_, variant) in &mut enm.variants {
                map_datatype_fields(exporter, types, &mut variant.fields)?;
            }
        }
        DataType::Tuple(tuple) => {
            for element in &mut tuple.elements {
                *element = map_datatype_format(exporter, types, element)?;
            }
        }
        DataType::Reference(Reference::Named(reference)) => {
            for (_, generic) in &mut reference.generics {
                *generic = map_datatype_format(exporter, types, generic)?;
            }
        }
        DataType::Reference(Reference::Generic(_) | Reference::Opaque(_)) => {}
    }

    Ok(dt)
}

fn map_datatype_fields(exporter: &Go, types: &Types, fields: &mut Fields) -> Result<(), Error> {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in &mut unnamed.fields {
                if let Some(ty) = field.ty.as_mut() {
                    *ty = map_datatype_format(exporter, types, ty)?;
                }
            }
        }
        Fields::Named(named) => {
            for (_, field) in &mut named.fields {
                if let Some(ty) = field.ty.as_mut() {
                    *ty = map_datatype_format(exporter, types, ty)?;
                }
            }
        }
    }

    Ok(())
}

fn map_types_for_datatype_format<'a>(
    exporter: &Go,
    types: &'a Types,
) -> Result<Cow<'a, Types>, Error> {
    if exporter.format.is_none() {
        return Ok(Cow::Borrowed(types));
    }

    let mut mapped_types = types.clone();
    let mut map_err = None;
    mapped_types.iter_mut(|ndt| {
        if map_err.is_some() {
            return;
        }

        match map_datatype_format(exporter, types, &ndt.ty) {
            Ok(mapped) => ndt.ty = mapped,
            Err(err) => map_err = Some(err),
        }
    });

    if let Some(err) = map_err {
        return Err(err);
    }

    Ok(Cow::Owned(mapped_types))
}
