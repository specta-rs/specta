use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    path::Path,
};

use specta::{
    Types,
    datatype::{
        DataType, Deprecated, Enum, Field, Fields, Generic, NamedDataType, NamedReferenceType,
        Primitive, Reference, Struct,
    },
};

use crate::{Error, Java, Layout, OptionalStyle, reserved_names::is_keyword};

pub(crate) fn flat_file(java: &Java, types: &Types) -> Result<String, Error> {
    validate_package(java.package.as_deref())?;
    let class_name = class_name(java)?;
    validate_unique_names(java, types, false)?;
    for ndt in types.into_sorted_iter().filter(|ndt| ndt.ty.is_some()) {
        if rendered_type_name(java, ndt)? == class_name {
            return Err(Error::duplicate_type(
                class_name,
                "flat-file wrapper",
                rust_path(ndt),
            ));
        }
    }

    let mut out = file_header(java, java.package.as_deref());
    writeln!(out, "public final class {class_name} {{").expect("writing to String cannot fail");
    writeln!(out, "    private {class_name}() {{}}").expect("writing to String cannot fail");

    let mut first = true;
    for ndt in types.into_sorted_iter().filter(|ndt| ndt.ty.is_some()) {
        if first {
            out.push('\n');
            first = false;
        } else {
            out.push_str("\n\n");
        }
        let declaration = declaration(java, types, ndt, true, false)
            .map_err(|err| err.with_path(rust_path(ndt)).with_named_datatype(ndt))?;
        out.push_str(&indent(&declaration, 1));
    }
    for raw in &java.raw {
        if !raw.is_empty() {
            out.push_str("\n\n");
            out.push_str(&indent(raw.trim_end(), 1));
        }
    }
    out.push_str("\n}\n");
    Ok(out)
}

pub(crate) fn class_name(java: &Java) -> Result<String, Error> {
    type_identifier(&java.class_name, "wrapper class")
}

pub(crate) fn inline(java: &Java, types: &Types, value: &DataType) -> Result<String, Error> {
    let generic_scope = Vec::new();
    datatype(
        &Context {
            java,
            types,
            generic_scope: &generic_scope,
            current_module: "",
            qualified_references: false,
            nested_type_scope: Vec::new(),
        },
        value,
        "inline",
    )
}

pub(crate) fn files(java: &Java, root: &Path, types: &Types) -> Result<(), Error> {
    validate_package(java.package.as_deref())?;
    validate_unique_names(java, types, true)?;
    let mut outputs = BTreeMap::new();

    for ndt in types.into_sorted_iter().filter(|ndt| ndt.ty.is_some()) {
        let module_package = module_package(&ndt.module_path)?;
        let package = join_package(java.package.as_deref(), &module_package);
        let directory = package
            .as_deref()
            .map(|package| {
                package
                    .split('.')
                    .fold(root.to_path_buf(), |path, part| path.join(part))
            })
            .unwrap_or_else(|| root.to_path_buf());
        let name = type_identifier(&ndt.name, &rust_path(ndt))?;
        let mut out = file_header(java, package.as_deref());
        out.push_str(
            &declaration(java, types, ndt, false, true)
                .map_err(|err| err.with_path(rust_path(ndt)).with_named_datatype(ndt))?,
        );
        out.push('\n');
        let path = directory.join(format!("{name}.java"));
        if outputs.insert(path, out).is_some() {
            return Err(Error::duplicate_type(name, rust_path(ndt), rust_path(ndt)));
        }
    }

    if !java.raw.is_empty() {
        let package = java.package.as_deref();
        let directory = package
            .map(|package| {
                package
                    .split('.')
                    .fold(root.to_path_buf(), |path, part| path.join(part))
            })
            .unwrap_or_else(|| root.to_path_buf());
        let name = class_name(java)?;
        let mut out = file_header(java, package);
        writeln!(out, "public final class {name} {{").expect("writing to String cannot fail");
        writeln!(out, "    private {name}() {{}}").expect("writing to String cannot fail");
        for raw in &java.raw {
            out.push('\n');
            out.push_str(&indent(raw.trim_end(), 1));
            out.push('\n');
        }
        out.push_str("}\n");
        let path = directory.join(format!("{name}.java"));
        if outputs.insert(path, out).is_some() {
            return Err(Error::duplicate_type(
                name,
                "generated Java type",
                "raw Java wrapper",
            ));
        }
    }

    std::fs::create_dir_all(root)
        .map_err(|source| Error::create_dir(root.to_path_buf(), source))?;
    remove_stale_generated_files(root, &outputs, &generated_marker(java))?;
    for (path, source) in outputs {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|source| Error::create_dir(parent.to_path_buf(), source))?;
        }
        std::fs::write(&path, source).map_err(|source| Error::write_file(path, source))?;
    }
    Ok(())
}

const GENERATED_MARKER: &str = "// @generated by specta-java";

fn remove_stale_generated_files(
    directory: &Path,
    expected: &BTreeMap<std::path::PathBuf, String>,
    marker: &str,
) -> Result<(), Error> {
    for entry in std::fs::read_dir(directory)
        .map_err(|source| Error::read_dir(directory.to_path_buf(), source))?
    {
        let entry = entry.map_err(|source| Error::read_dir(directory.to_path_buf(), source))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| Error::read_dir(directory.to_path_buf(), source))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            remove_stale_generated_files(&path, expected, marker)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
            && !expected.contains_key(&path)
        {
            let source = std::fs::read_to_string(&path)
                .map_err(|source| Error::read_file(path.clone(), source))?;
            if source.lines().any(|line| line == marker) {
                std::fs::remove_file(&path)
                    .map_err(|source| Error::remove_file(path.clone(), source))?;
            }
        }
    }
    Ok(())
}

fn file_header(java: &Java, package: Option<&str>) -> String {
    let mut out = String::new();
    if !java.header.is_empty() {
        out.push_str(java.header.trim_end());
        out.push('\n');
    }
    out.push_str(&generated_marker(java));
    out.push('\n');
    if let Some(package) = package.filter(|package| !package.is_empty()) {
        if !out.is_empty() {
            out.push('\n');
        }
        writeln!(out, "package {package};").expect("writing to String cannot fail");
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

fn generated_marker(java: &Java) -> String {
    java.package
        .as_deref()
        .map(|package| format!("{GENERATED_MARKER} (package: {package})"))
        .unwrap_or_else(|| GENERATED_MARKER.to_string())
}

fn declaration(
    java: &Java,
    types: &Types,
    ndt: &NamedDataType,
    _nested: bool,
    qualified_references: bool,
) -> Result<String, Error> {
    let name = rendered_type_name(java, ndt)?;
    let visibility = "public ";
    let generic_scope = generic_bindings(java, types, ndt)?;
    let generics = generic_definitions(&generic_scope);
    let ctx = Context {
        java,
        types,
        generic_scope: &generic_scope,
        current_module: &ndt.module_path,
        qualified_references,
        nested_type_scope: Vec::new(),
    };

    let mut out = String::new();
    javadoc(&mut out, &ndt.docs, ndt.deprecated.as_ref(), 0);
    if ndt.deprecated.is_some() {
        out.push_str("@Deprecated\n");
    }
    match ndt
        .ty
        .as_ref()
        .expect("callers filter types without definitions")
    {
        DataType::Struct(value) => {
            render_record(&mut out, &ctx, visibility, &name, &generics, value, &name)?
        }
        DataType::Enum(value)
            if ndt.generics.is_empty()
                && (is_unit_enum(value) || resolved_string_enum(value).is_some()) =>
        {
            render_unit_enum(&mut out, visibility, &name, value, &name)?
        }
        DataType::Enum(value) => {
            render_tagged_enum(&mut out, &ctx, visibility, &name, &generics, value, &name)?
        }
        ty => {
            let rendered = datatype(&ctx, ty, &name)?;
            write!(
                out,
                "{visibility}record {name}{generics}({rendered} value) {{}}"
            )
            .expect("writing to String cannot fail");
        }
    }
    Ok(out)
}

fn render_record(
    out: &mut String,
    ctx: &Context<'_>,
    visibility: &str,
    name: &str,
    generics: &str,
    value: &Struct,
    path: &str,
) -> Result<(), Error> {
    let fields = fields(ctx, &value.fields, path)?;
    validate_nested_declarations(&fields, name, path)?;
    if fields.is_empty() {
        write!(out, "{visibility}record {name}{generics}() {{}}")
            .expect("writing to String cannot fail");
        return Ok(());
    }

    writeln!(out, "{visibility}record {name}{generics}(").expect("writing to String cannot fail");
    for (index, field) in fields.iter().enumerate() {
        if !field.docs.is_empty() || field.deprecated.is_some() {
            javadoc(out, field.docs, field.deprecated, 1);
        }
        write!(out, "    {} {}", field.ty, field.name).expect("writing to String cannot fail");
        if index + 1 != fields.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push(')');
    render_record_body(out, &fields, 0);
    Ok(())
}

fn render_unit_enum(
    out: &mut String,
    visibility: &str,
    name: &str,
    value: &Enum,
    path: &str,
) -> Result<(), Error> {
    writeln!(out, "{visibility}enum {name} {{").expect("writing to String cannot fail");
    let raw_values = resolved_string_enum(value);
    let variants = value
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .collect::<Vec<_>>();
    validate_names(
        variants
            .iter()
            .map(|(variant, _)| value_identifier(variant, path)),
        path,
    )?;
    let mut backing_name = "value".to_string();
    let variant_names = variants
        .iter()
        .map(|(variant, _)| value_identifier(variant, path))
        .collect::<Result<BTreeSet<_>, _>>()?;
    while variant_names.contains(&backing_name) {
        backing_name.push('_');
    }
    for (index, (variant_name, variant)) in variants.iter().enumerate() {
        javadoc(out, &variant.docs, variant.deprecated.as_ref(), 1);
        if variant.deprecated.is_some() {
            out.push_str("    @Deprecated\n");
        }
        let variant_name = value_identifier(variant_name, &format!("{path}.{variant_name}"))?;
        write!(out, "    {variant_name}").expect("writing to String cannot fail");
        if let Some(raw_values) = &raw_values {
            let raw_value = raw_values[index].1;
            write!(out, "(\"{}\")", escape_java_string(raw_value))
                .expect("writing to String cannot fail");
        }
        if index + 1 != variants.len() {
            out.push(',');
        } else if raw_values.is_some() {
            out.push(';');
        }
        out.push('\n');
    }
    if raw_values.is_some() {
        writeln!(
            out,
            "\n    private final java.lang.String {backing_name};\n"
        )
        .expect("writing to String cannot fail");
        writeln!(out, "    {name}(java.lang.String value) {{")
            .expect("writing to String cannot fail");
        writeln!(out, "        this.{backing_name} = value;\n    }}\n")
            .expect("writing to String cannot fail");
        writeln!(
            out,
            "    public java.lang.String value() {{\n        return {backing_name};\n    }}"
        )
        .expect("writing to String cannot fail");
    }
    out.push('}');
    Ok(())
}

fn render_tagged_enum(
    out: &mut String,
    ctx: &Context<'_>,
    visibility: &str,
    name: &str,
    generics: &str,
    value: &Enum,
    path: &str,
) -> Result<(), Error> {
    let variants = value
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .collect::<Vec<_>>();
    if variants.is_empty() {
        return Err(Error::unsupported(
            path,
            "Java sealed interfaces require at least one non-skipped enum variant",
        ));
    }
    validate_names(
        variants
            .iter()
            .map(|(variant, _)| type_identifier(variant, path)),
        path,
    )?;
    if variants
        .iter()
        .map(|(variant, _)| type_identifier(variant, path))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|variant| variant == name)
    {
        return Err(Error::duplicate_type(name, path, path));
    }
    let permits = variants
        .iter()
        .map(|(variant, _)| type_identifier(variant, &format!("{path}.{variant}")))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|variant| format!("{name}.{variant}"))
        .collect::<Vec<_>>()
        .join(", ");
    write!(out, "{visibility}sealed interface {name}{generics}")
        .expect("writing to String cannot fail");
    if !permits.is_empty() {
        write!(out, " permits {permits}").expect("writing to String cannot fail");
    }
    out.push_str(" {\n");

    for (index, (variant_name, variant)) in variants.iter().enumerate() {
        if index != 0 {
            out.push('\n');
        }
        javadoc(out, &variant.docs, variant.deprecated.as_ref(), 1);
        if variant.deprecated.is_some() {
            out.push_str("    @Deprecated\n");
        }
        let java_name = type_identifier(variant_name, &format!("{path}.{variant_name}"))?;
        let fields = fields(ctx, &variant.fields, &format!("{path}.{variant_name}"))?;
        validate_nested_declarations(&fields, &java_name, path)?;
        if fields.is_empty() {
            write!(
                out,
                "    record {java_name}{generics}() implements {name}{generics} {{}}"
            )
            .expect("writing to String cannot fail");
            out.push('\n');
            continue;
        }
        writeln!(out, "    record {java_name}{generics}(").expect("writing to String cannot fail");
        for (field_index, field) in fields.iter().enumerate() {
            javadoc(out, field.docs, field.deprecated, 2);
            write!(out, "        {} {}", field.ty, field.name)
                .expect("writing to String cannot fail");
            if field_index + 1 != fields.len() {
                out.push(',');
            }
            out.push('\n');
        }
        write!(out, "    ) implements {name}{generics}").expect("writing to String cannot fail");
        render_record_body(out, &fields, 1);
        out.push('\n');
    }
    out.push('}');
    Ok(())
}

struct RenderedField<'a> {
    name: String,
    ty: String,
    docs: &'a str,
    deprecated: Option<&'a Deprecated>,
    nested: Option<String>,
}

fn render_record_body(out: &mut String, fields: &[RenderedField<'_>], level: usize) {
    let nested = fields
        .iter()
        .filter_map(|field| field.nested.as_deref())
        .collect::<Vec<_>>();
    if nested.is_empty() {
        out.push_str(" {}");
        return;
    }
    out.push_str(" {\n");
    for (index, declaration) in nested.iter().enumerate() {
        if index != 0 {
            out.push('\n');
        }
        out.push_str(&indent(declaration, level + 1));
        out.push('\n');
    }
    out.push_str(&"    ".repeat(level));
    out.push('}');
}

fn fields<'a>(
    ctx: &Context<'_>,
    fields: &'a Fields,
    path: &str,
) -> Result<Vec<RenderedField<'a>>, Error> {
    let mut rendered = match fields {
        Fields::Unit => Ok(Vec::new()),
        Fields::Unnamed(fields) => fields
            .fields
            .iter()
            .enumerate()
            .filter_map(|(index, field)| field.ty.as_ref().map(|ty| (index, field, ty)))
            .map(|(index, field, ty)| {
                let name = format!("field{index}");
                let field_path = format!("{path}.{index}");
                let (ty, nested) = field_datatype(ctx, ty, &name, &field_path)?;
                Ok(RenderedField {
                    ty: optional_field_type(ctx.java, field, ty),
                    name,
                    docs: &field.docs,
                    deprecated: field.deprecated.as_ref(),
                    nested,
                })
            })
            .collect(),
        Fields::Named(fields) => fields
            .fields
            .iter()
            .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, field, ty)))
            .map(|(name, field, ty)| rendered_field(ctx, path, name, field, ty))
            .collect(),
    }?;
    let mut counts = BTreeMap::<String, usize>::new();
    let mut names = BTreeSet::new();
    for field in &mut rendered {
        if names.insert(field.name.clone()) {
            counts.insert(field.name.clone(), 1);
            continue;
        }
        if field.nested.is_some() {
            return Err(Error::duplicate_type(field.name.clone(), path, path));
        }
        let base = field.name.clone();
        let count = counts.entry(base.clone()).or_insert(1);
        loop {
            *count += 1;
            let candidate = format!("{base}_{count}");
            if names.insert(candidate.clone()) {
                field.name = candidate;
                break;
            }
        }
    }
    Ok(rendered)
}

fn rendered_field<'a>(
    ctx: &Context<'_>,
    path: &str,
    name: &str,
    field: &'a Field,
    ty: &DataType,
) -> Result<RenderedField<'a>, Error> {
    let field_path = format!("{path}.{name}");
    let java_name = value_identifier(name, &field_path)?;
    let (ty, nested) = field_datatype(ctx, ty, &java_name, &field_path)?;
    Ok(RenderedField {
        name: java_name,
        ty: optional_field_type(ctx.java, field, ty),
        docs: &field.docs,
        deprecated: field.deprecated.as_ref(),
        nested,
    })
}

fn optional_field_type(java: &Java, field: &Field, ty: String) -> String {
    if field.optional && !ty.starts_with("java.util.Optional<") {
        optional_type(java, ty)
    } else {
        ty
    }
}

fn field_datatype(
    ctx: &Context<'_>,
    datatype_value: &DataType,
    field_name: &str,
    path: &str,
) -> Result<(String, Option<String>), Error> {
    if string_literal_raw_value(datatype_value).is_some() {
        return Ok(("java.lang.String".to_string(), None));
    }
    if let DataType::Nullable(inner) = datatype_value {
        let (inner, nested) = field_datatype(ctx, inner, field_name, path)?;
        return Ok((optional_type(ctx.java, inner), nested));
    }
    if let DataType::Reference(Reference::Named(reference)) = datatype_value
        && let NamedReferenceType::Inline { dt, .. } = &reference.inner
    {
        return field_datatype(ctx, dt, field_name, path);
    }
    let type_name = nested_type_identifier(ctx, field_name, path)?;
    let nested_ctx = ctx.with_nested_type(type_name.clone());
    let generic_names = ctx
        .generic_scope
        .iter()
        .map(|generic| generic.identifier.as_str())
        .collect::<Vec<_>>();
    let generics = if generic_names.is_empty() {
        String::new()
    } else {
        format!("<{}>", generic_names.join(", "))
    };
    let mut nested = String::new();
    match datatype_value {
        DataType::List(list) => {
            let (item, nested) = field_datatype(ctx, &list.ty, &format!("{field_name}Item"), path)?;
            return Ok((format!("java.util.List<{item}>"), nested));
        }
        DataType::Map(map) => {
            let (key, key_nested) =
                field_datatype(ctx, map.key_ty(), &format!("{field_name}Key"), path)?;
            let (value, value_nested) =
                field_datatype(ctx, map.value_ty(), &format!("{field_name}Value"), path)?;
            let nested = [key_nested, value_nested]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("\n\n");
            return Ok((
                format!("java.util.Map<{key}, {value}>"),
                (!nested.is_empty()).then_some(nested),
            ));
        }
        DataType::Tuple(tuple) if tuple.elements.len() > 1 => {
            let mut tuple_fields = Vec::new();
            for (index, element) in tuple.elements.iter().enumerate() {
                let name = format!("field{index}");
                let (ty, nested) = field_datatype(&nested_ctx, element, &name, path)?;
                tuple_fields.push(RenderedField {
                    name,
                    ty,
                    docs: "",
                    deprecated: None,
                    nested,
                });
            }
            writeln!(nested, "public record {type_name}{generics}(")
                .expect("writing to String cannot fail");
            for (index, field) in tuple_fields.iter().enumerate() {
                write!(nested, "    {} {}", field.ty, field.name)
                    .expect("writing to String cannot fail");
                if index + 1 != tuple_fields.len() {
                    nested.push(',');
                }
                nested.push('\n');
            }
            nested.push(')');
            render_record_body(&mut nested, &tuple_fields, 0);
        }
        DataType::Struct(value) => {
            render_record(
                &mut nested,
                &nested_ctx,
                "public ",
                &type_name,
                &generics,
                value,
                path,
            )?;
        }
        DataType::Enum(value) if is_unit_enum(value) || resolved_string_enum(value).is_some() => {
            render_unit_enum(&mut nested, "public ", &type_name, value, path)?;
            return Ok((type_name, Some(nested)));
        }
        DataType::Enum(value) => {
            render_tagged_enum(
                &mut nested,
                &nested_ctx,
                "public ",
                &type_name,
                &generics,
                value,
                path,
            )?;
        }
        _ => return Ok((datatype(ctx, datatype_value, path)?, None)),
    }
    Ok((format!("{type_name}{generics}"), Some(nested)))
}

fn nested_type_identifier(ctx: &Context<'_>, name: &str, path: &str) -> Result<String, Error> {
    let base = type_identifier(name, path)?;
    let mut occupied = ctx
        .types
        .into_sorted_iter()
        .filter(|datatype| datatype.ty.is_some())
        .map(|datatype| rendered_type_name(ctx.java, datatype))
        .collect::<Result<BTreeSet<_>, _>>()?;
    occupied.extend(
        ctx.generic_scope
            .iter()
            .map(|generic| generic.identifier.clone()),
    );
    occupied.extend(ctx.nested_type_scope.iter().cloned());
    occupied.insert(class_name(ctx.java)?);
    if !occupied.contains(&base) {
        return Ok(base);
    }

    let mut identifier = format!("{base}Inline");
    let mut suffix = 2;
    while occupied.contains(&identifier) {
        identifier = format!("{base}Inline{suffix}");
        suffix += 1;
    }
    Ok(identifier)
}

struct Context<'a> {
    java: &'a Java,
    types: &'a Types,
    generic_scope: &'a [GenericBinding],
    current_module: &'a str,
    qualified_references: bool,
    nested_type_scope: Vec<String>,
}

impl<'a> Context<'a> {
    fn with_nested_type(&self, name: String) -> Context<'a> {
        let mut nested_type_scope = self.nested_type_scope.clone();
        nested_type_scope.push(name);
        Context {
            java: self.java,
            types: self.types,
            generic_scope: self.generic_scope,
            current_module: self.current_module,
            qualified_references: self.qualified_references,
            nested_type_scope,
        }
    }
}

fn datatype(ctx: &Context<'_>, ty: &DataType, path: &str) -> Result<String, Error> {
    if string_literal_raw_value(ty).is_some() {
        return Ok("java.lang.String".to_string());
    }
    Ok(match ty {
        DataType::Primitive(primitive) => primitive_type(primitive).to_string(),
        DataType::List(list) => format!("java.util.List<{}>", datatype(ctx, &list.ty, path)?),
        DataType::Map(map) => format!(
            "java.util.Map<{}, {}>",
            datatype(ctx, map.key_ty(), &format!("{path}.key"))?,
            datatype(ctx, map.value_ty(), &format!("{path}.value"))?
        ),
        DataType::Nullable(inner) => optional_type(ctx.java, datatype(ctx, inner, path)?),
        DataType::Tuple(tuple) if tuple.elements.is_empty() => "java.lang.Void".to_string(),
        DataType::Tuple(tuple) if tuple.elements.len() == 1 => {
            datatype(ctx, &tuple.elements[0], path)?
        }
        DataType::Tuple(_) => "java.util.List<java.lang.Object>".to_string(),
        DataType::Struct(_) => "java.util.Map<java.lang.String, java.lang.Object>".to_string(),
        DataType::Enum(_) => "java.lang.Object".to_string(),
        DataType::Intersection(_) => {
            return Err(Error::unsupported(
                path,
                "Java has no structural intersection value type",
            ));
        }
        DataType::Generic(generic) => ctx
            .generic_scope
            .iter()
            .find(|candidate| candidate.reference == *generic)
            .map(|candidate| candidate.identifier.clone())
            .ok_or_else(|| {
                Error::unsupported(path, "generic is not declared by the containing type")
            })?,
        DataType::Reference(Reference::Opaque(reference)) => {
            return Err(Error::unsupported_opaque(path, reference.clone()));
        }
        DataType::Reference(Reference::Named(reference)) => {
            let ndt = ctx
                .types
                .get(reference)
                .ok_or_else(|| Error::dangling_reference(path, format!("{reference:?}")))?;
            match &reference.inner {
                NamedReferenceType::Inline { dt, .. } => datatype(ctx, dt, path)?,
                NamedReferenceType::Recursive(cycle) => {
                    return Err(Error::recursive_inline(path, rust_path(ndt), cycle.clone()));
                }
                NamedReferenceType::Reference { generics, .. } => {
                    if ndt.ty.is_none() {
                        return Err(Error::unsupported(
                            path,
                            format!(
                                "named type '{}' has no definition to export",
                                rust_path(ndt)
                            ),
                        ));
                    }
                    let mut name = rendered_type_name(ctx.java, ndt)?;
                    if ctx.qualified_references && ndt.module_path.as_ref() != ctx.current_module {
                        let package = join_package(
                            ctx.java.package.as_deref(),
                            &module_package(&ndt.module_path)?,
                        );
                        if let Some(package) = package {
                            name = format!("{package}.{name}");
                        } else if !ctx.current_module.is_empty() {
                            return Err(Error::unsupported(
                                path,
                                "Java types in named packages cannot reference a type in the default package; configure a base package",
                            ));
                        }
                    }
                    if !generics.is_empty() {
                        let values = generics
                            .iter()
                            .map(|(_, ty)| datatype(ctx, ty, path))
                            .collect::<Result<Vec<_>, _>>()?;
                        name.push('<');
                        name.push_str(&values.join(", "));
                        name.push('>');
                    }
                    name
                }
            }
        }
    })
}

fn optional_type(java: &Java, inner: String) -> String {
    match java.optionals {
        OptionalStyle::Nullable => inner,
        OptionalStyle::Optional => format!("java.util.Optional<{inner}>"),
    }
}

fn primitive_type(primitive: &Primitive) -> &'static str {
    match primitive {
        Primitive::i8 => "java.lang.Byte",
        Primitive::u8 | Primitive::i16 => "java.lang.Short",
        Primitive::u16 | Primitive::i32 => "java.lang.Integer",
        Primitive::u32 | Primitive::i64 => "java.lang.Long",
        Primitive::i128
        | Primitive::u64
        | Primitive::u128
        | Primitive::isize
        | Primitive::usize => "java.math.BigInteger",
        Primitive::f16 | Primitive::f32 => "java.lang.Float",
        Primitive::f64 => "java.lang.Double",
        Primitive::f128 => "java.math.BigDecimal",
        Primitive::bool => "java.lang.Boolean",
        Primitive::char => "java.lang.String",
        Primitive::str => "java.lang.String",
    }
}

struct GenericBinding {
    reference: Generic,
    identifier: String,
}

fn generic_bindings(
    java: &Java,
    types: &Types,
    ndt: &NamedDataType,
) -> Result<Vec<GenericBinding>, Error> {
    let base_names = ndt
        .generics
        .iter()
        .map(|generic| type_identifier(&generic.name, "generic parameter"))
        .collect::<Result<Vec<_>, _>>()?;
    validate_names(base_names.iter().cloned().map(Ok), &rust_path(ndt))?;

    let mut occupied = types
        .into_sorted_iter()
        .filter(|datatype| datatype.ty.is_some())
        .map(|datatype| rendered_type_name(java, datatype))
        .collect::<Result<BTreeSet<_>, _>>()?;
    occupied.insert(class_name(java)?);

    Ok(ndt
        .generics
        .iter()
        .zip(base_names)
        .map(|(generic, base)| {
            let mut identifier = base.clone();
            if occupied.contains(&identifier) {
                identifier = format!("{base}Type");
                let mut suffix = 2;
                while occupied.contains(&identifier) {
                    identifier = format!("{base}Type{suffix}");
                    suffix += 1;
                }
            }
            occupied.insert(identifier.clone());
            GenericBinding {
                reference: generic.reference(),
                identifier,
            }
        })
        .collect())
}

fn generic_definitions(generics: &[GenericBinding]) -> String {
    if generics.is_empty() {
        return String::new();
    }
    format!(
        "<{}>",
        generics
            .iter()
            .map(|generic| generic.identifier.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn validate_nested_declarations(
    fields: &[RenderedField<'_>],
    enclosing_name: &str,
    path: &str,
) -> Result<(), Error> {
    let mut seen = BTreeMap::<String, ()>::new();
    seen.insert(enclosing_name.to_string(), ());
    for source in fields.iter().filter_map(|field| field.nested.as_deref()) {
        for line in source.lines() {
            let name = ["public record ", "public enum ", "public sealed interface "]
                .into_iter()
                .find_map(|prefix| line.strip_prefix(prefix))
                .and_then(|rest| rest.split(['<', '(', ' ', '{']).next());
            if let Some(name) = name
                && seen.insert(name.to_string(), ()).is_some()
            {
                return Err(Error::duplicate_type(name, path, path));
            }
        }
    }
    Ok(())
}

fn is_unit_enum(value: &Enum) -> bool {
    value
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .all(|(_, variant)| matches!(variant.fields, Fields::Unit))
}

fn string_literal_raw_value(datatype: &DataType) -> Option<&str> {
    let DataType::Enum(value) = datatype else {
        return None;
    };
    if !value
        .attributes
        .contains_key("specta_serde:enum_repr_rewritten")
    {
        return None;
    }
    let [(raw_value, variant)] = value.variants.as_slice() else {
        return None;
    };
    match &variant.fields {
        Fields::Unit => Some(raw_value),
        Fields::Unnamed(fields) => {
            let [field] = fields.fields.as_slice() else {
                return None;
            };
            string_literal_raw_value(field.ty.as_ref()?)
        }
        Fields::Named(fields) => {
            let [(_, field)] = fields.fields.as_slice() else {
                return None;
            };
            string_literal_raw_value(field.ty.as_ref()?)
        }
    }
}

fn resolved_string_enum(value: &Enum) -> Option<Vec<(&str, &str)>> {
    let values = value
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .map(|(variant_name, variant)| {
            let datatype = match &variant.fields {
                Fields::Unnamed(fields) => {
                    let [field] = fields.fields.as_slice() else {
                        return None;
                    };
                    field.ty.as_ref()?
                }
                Fields::Unit | Fields::Named(_) => return None,
            };
            string_literal_raw_value(datatype).map(|raw| (variant_name.as_ref(), raw))
        })
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then_some(values)
}

fn escape_java_string(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            '\u{0008}' => "\\b".chars().collect(),
            '\u{000c}' => "\\f".chars().collect(),
            character if (character as u32) <= 0xff && character.is_control() => {
                format!("\\{:03o}", character as u32).chars().collect()
            }
            character => vec![character],
        })
        .collect()
}

fn javadoc(out: &mut String, docs: &str, deprecated: Option<&Deprecated>, level: usize) {
    if docs.trim().is_empty() && deprecated.is_none() {
        return;
    }
    let indent = "    ".repeat(level);
    writeln!(out, "{indent}/**").expect("writing to String cannot fail");
    for line in docs.lines() {
        writeln!(out, "{indent} * {}", escape_javadoc(line.trim_start()))
            .expect("writing to String cannot fail");
    }
    if let Some(deprecated) = deprecated {
        let note = deprecated.note.as_deref().unwrap_or("Deprecated.");
        writeln!(out, "{indent} * @deprecated {}", escape_javadoc(note))
            .expect("writing to String cannot fail");
    }
    writeln!(out, "{indent} */").expect("writing to String cannot fail");
}

fn escape_javadoc(value: &str) -> String {
    value.replace("*/", "*&#47;")
}

fn validate_unique_names(java: &Java, types: &Types, include_modules: bool) -> Result<(), Error> {
    let mut seen = BTreeMap::<String, String>::new();
    for ndt in types.into_sorted_iter().filter(|ndt| ndt.ty.is_some()) {
        let mut name =
            rendered_type_name(java, ndt).map_err(|error| error.with_named_datatype(ndt))?;
        if include_modules {
            name = format!("{}.{}", module_package(&ndt.module_path)?, name);
        }
        let path = rust_path(ndt);
        if let Some(first) = seen.insert(name.clone(), path.clone()) {
            return Err(Error::duplicate_type(name, first, path));
        }
    }
    Ok(())
}

fn rendered_type_name(java: &Java, ndt: &NamedDataType) -> Result<String, Error> {
    let name = type_identifier(&ndt.name, &rust_path(ndt))?;
    if java.layout != Layout::ModulePrefixedName || ndt.module_path.is_empty() {
        return Ok(name);
    }
    let prefix = ndt
        .module_path
        .split("::")
        .filter(|part| !part.is_empty())
        .map(|part| type_identifier(part, &ndt.module_path))
        .collect::<Result<String, _>>()?;
    Ok(format!("{prefix}{name}"))
}

fn validate_package(package: Option<&str>) -> Result<(), Error> {
    if let Some(package) = package {
        for part in package.split('.') {
            strict_identifier(part, package)?;
        }
    }
    Ok(())
}

fn module_package(module: &str) -> Result<String, Error> {
    module
        .split("::")
        .filter(|part| !part.is_empty())
        .map(|part| package_identifier(part, module))
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("."))
}

fn join_package(base: Option<&str>, module: &str) -> Option<String> {
    match (base.filter(|base| !base.is_empty()), module.is_empty()) {
        (Some(base), false) => Some(format!("{base}.{module}")),
        (Some(base), true) => Some(base.to_string()),
        (None, false) => Some(module.to_string()),
        (None, true) => None,
    }
}

fn type_identifier(name: &str, path: &str) -> Result<String, Error> {
    identifier(name, path, true)
}

fn value_identifier(name: &str, path: &str) -> Result<String, Error> {
    if is_strict_identifier(name) {
        let mut name = name.to_string();
        if is_forbidden_record_component(&name) {
            name.push('_');
        }
        return Ok(name);
    }
    let mut name = identifier(name, path, false)?;
    if is_forbidden_record_component(&name) {
        name.push('_');
    }
    Ok(name)
}

fn package_identifier(name: &str, path: &str) -> Result<String, Error> {
    let mut name = identifier(name, path, false)?.to_ascii_lowercase();
    if is_keyword(&name) {
        name.push('_');
    }
    Ok(name)
}

fn identifier(name: &str, path: &str, uppercase: bool) -> Result<String, Error> {
    if name.is_empty() {
        return Err(Error::invalid_identifier(path, name));
    }
    let mut out = String::with_capacity(name.len());
    let mut capitalize = uppercase;
    for character in name.chars() {
        if character == '_' || character == '-' || character == ' ' || character == '.' {
            capitalize = true;
            continue;
        }
        if out.is_empty() && character.is_numeric() {
            out.push('_');
        }
        let valid = character == '$' || character == '_' || character.is_alphanumeric();
        if !valid {
            capitalize = true;
            continue;
        }
        if capitalize {
            out.extend(character.to_uppercase());
            capitalize = false;
        } else {
            out.push(character);
        }
    }
    if out.is_empty() {
        out.push_str(if uppercase { "Type" } else { "field" });
    }
    if is_keyword(&out) {
        out.push('_');
    }
    Ok(out)
}

fn is_forbidden_record_component(name: &str) -> bool {
    matches!(
        name,
        "clone"
            | "finalize"
            | "getClass"
            | "hashCode"
            | "notify"
            | "notifyAll"
            | "toString"
            | "wait"
    )
}

fn is_strict_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|character| character == '$' || character == '_' || character.is_alphabetic())
        && characters
            .all(|character| character == '$' || character == '_' || character.is_alphanumeric())
        && !is_keyword(name)
}

fn strict_identifier(name: &str, path: &str) -> Result<(), Error> {
    is_strict_identifier(name)
        .then_some(())
        .ok_or_else(|| Error::invalid_identifier(path, name))
}

fn validate_names(
    names: impl IntoIterator<Item = Result<String, Error>>,
    path: &str,
) -> Result<(), Error> {
    let mut seen = BTreeMap::<String, ()>::new();
    for name in names {
        let name = name?;
        if seen.insert(name.clone(), ()).is_some() {
            return Err(Error::duplicate_type(name, path, path));
        }
    }
    Ok(())
}

fn rust_path(ndt: &NamedDataType) -> String {
    if ndt.module_path.is_empty() {
        ndt.name.to_string()
    } else {
        format!("{}::{}", ndt.module_path, ndt.name)
    }
}

fn indent(value: &str, levels: usize) -> String {
    let prefix = "    ".repeat(levels);
    value
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_java_safe() {
        assert_eq!(
            type_identifier("some_type", "test").expect("identifier should be valid"),
            "SomeType"
        );
        assert_eq!(
            value_identifier("some-field", "test").expect("identifier should be escaped"),
            "someField"
        );
        assert_eq!(
            value_identifier("class", "test").expect("keyword should be escaped"),
            "class_"
        );
        assert_eq!(
            value_identifier("not/valid", "test").expect("identifier should be escaped"),
            "notValid"
        );
    }

    #[test]
    fn javadoc_terminators_are_escaped() {
        assert_eq!(escape_javadoc("oops */ nope"), "oops *&#47; nope");
        assert_eq!(escape_java_string("\0\u{0008}\u{000c}"), "\\000\\b\\f");
    }
}
