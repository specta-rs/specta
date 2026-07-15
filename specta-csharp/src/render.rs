use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
};

use specta::{
    Format, Types,
    datatype::{
        DataType, Deprecated, Field, Fields, NamedDataType, NamedReference, NamedReferenceType,
        Primitive, Reference, Variant,
    },
};

use crate::{CSharp, Error, Layout};

pub(crate) fn export(
    exporter: &CSharp,
    types: &Types,
    format: &dyn Format,
) -> Result<String, Error> {
    let types = format_types(types, format)?;
    validate_names(exporter, &types)?;
    let ndts = types
        .into_sorted_iter()
        .filter(|ndt| is_emitted_named(ndt))
        .collect::<Vec<_>>();

    match exporter.layout {
        Layout::Namespaces => render_namespaces(exporter, &types, &ndts),
        Layout::FlatFile | Layout::ModulePrefixedName => {
            render_file(exporter, &types, &ndts, exporter.namespace.as_ref(), true)
        }
        Layout::Files => Err(Error::ExportRequiresExportTo(Layout::Files)),
    }
}

pub(crate) fn export_to(
    exporter: &CSharp,
    path: &Path,
    types: &Types,
    format: &dyn Format,
) -> Result<(), Error> {
    if exporter.layout != Layout::Files {
        let output = export(exporter, types, format)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| Error::io(parent, err))?;
        }
        return std::fs::write(path, output).map_err(|err| Error::io(path, err));
    }

    let types = format_types(types, format)?;
    validate_names(exporter, &types)?;

    let mut files = Vec::new();
    for ndt in types.into_sorted_iter().filter(|ndt| is_emitted_named(ndt)) {
        let module = module_segments(&ndt.module_path);
        let mut file_path = module
            .iter()
            .fold(path.to_path_buf(), |path, segment| path.join(segment));
        file_path.push(format!("{}.cs", exported_name(exporter, ndt)));
        let namespace = joined_namespace(exporter.namespace.as_ref(), &module);
        let output = render_file(exporter, &types, &[ndt], &namespace, false)?;
        files.push((file_path, output));
    }
    if !exporter.raw.is_empty() {
        let file_path = path.join("Specta.g.cs");
        let output = render_file(exporter, &types, &[], exporter.namespace.as_ref(), true)?;
        files.push((file_path, output));
    }

    reject_symlink_components(path)?;
    for (file_path, _) in &files {
        reject_symlink_components(file_path)?;
    }
    std::fs::create_dir_all(path).map_err(|err| Error::io(path, err))?;
    write_generated_files(path, &files)?;
    let expected = files.into_iter().map(|(path, _)| path).collect::<Vec<_>>();
    remove_stale_generated_files(path, &expected)
}

#[cfg(unix)]
fn write_generated_files(root: &Path, files: &[(PathBuf, String)]) -> Result<(), Error> {
    use std::io::Write;

    use rustix::fs::{Mode, OFlags, mkdirat, open, openat};

    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let root_fd =
        open(root, directory_flags, Mode::empty()).map_err(|err| Error::io(root, err.into()))?;
    for (file_path, output) in files {
        let relative = file_path
            .strip_prefix(root)
            .expect("generated file paths are rooted in the output directory");
        let mut components = relative.components().peekable();
        let mut directory = root_fd.try_clone().map_err(|err| Error::io(root, err))?;
        while let Some(component) = components.next() {
            let std::path::Component::Normal(name) = component else {
                unreachable!("generated file paths contain only validated normal components")
            };
            if components.peek().is_none() {
                let fd = openat(
                    &directory,
                    name,
                    OFlags::WRONLY
                        | OFlags::CREATE
                        | OFlags::TRUNC
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::from_bits_truncate(0o666),
                )
                .map_err(|err| Error::io(file_path, err.into()))?;
                let mut file = std::fs::File::from(fd);
                file.write_all(output.as_bytes())
                    .map_err(|err| Error::io(file_path, err))?;
                continue;
            }

            directory = match openat(&directory, name, directory_flags, Mode::empty()) {
                Ok(directory) => directory,
                Err(rustix::io::Errno::NOENT) => {
                    mkdirat(&directory, name, Mode::from_bits_truncate(0o777))
                        .map_err(|err| Error::io(file_path, err.into()))?;
                    openat(&directory, name, directory_flags, Mode::empty())
                        .map_err(|err| Error::io(file_path, err.into()))?
                }
                Err(err) => return Err(Error::io(file_path, err.into())),
            };
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn write_generated_files(_root: &Path, files: &[(PathBuf, String)]) -> Result<(), Error> {
    for (file_path, output) in files {
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| Error::io(parent, err))?;
        }
        reject_symlink_components(file_path)?;
        std::fs::write(file_path, output).map_err(|err| Error::io(file_path, err))?;
    }
    Ok(())
}

fn reject_symlink_components(path: &Path) -> Result<(), Error> {
    for component in path.ancestors().collect::<Vec<_>>().into_iter().rev() {
        match std::fs::symlink_metadata(component) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(Error::io(
                    component,
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "generated output path contains a symbolic link",
                    ),
                ));
            }
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(Error::io(component, err)),
        }
    }
    Ok(())
}

pub(crate) fn render_datatype(
    exporter: &CSharp,
    types: &Types,
    ty: &DataType,
) -> Result<String, Error> {
    let mut out = String::new();
    datatype(&mut out, exporter, types, ty, "datatype")?;
    Ok(out)
}

pub(crate) fn render_named_types(
    exporter: &CSharp,
    types: &Types,
    ndts: Vec<&NamedDataType>,
) -> Result<String, Error> {
    render_file(exporter, types, &ndts, exporter.namespace.as_ref(), true)
}

fn render_namespaces(
    exporter: &CSharp,
    types: &Types,
    ndts: &[&NamedDataType],
) -> Result<String, Error> {
    let mut groups: BTreeMap<Vec<String>, Vec<&NamedDataType>> = BTreeMap::new();
    for ndt in ndts {
        groups
            .entry(module_segments(&ndt.module_path))
            .or_default()
            .push(ndt);
    }

    let mut out = file_header(exporter);
    for (index, (module, group)) in groups.into_iter().enumerate() {
        if index != 0 || !out.is_empty() {
            out.push('\n');
        }
        let namespace = joined_namespace(exporter.namespace.as_ref(), &module);
        if !namespace.is_empty() {
            out.push_str("namespace ");
            out.push_str(&namespace);
            out.push_str("\n{\n");
        }
        let base_indent = if namespace.is_empty() {
            ""
        } else {
            exporter.indent.as_ref()
        };
        for (type_index, ndt) in group.iter().enumerate() {
            if type_index != 0 {
                out.push('\n');
            }
            render_named(&mut out, exporter, types, ndt, base_indent)?;
        }
        if !namespace.is_empty() {
            out.push_str("}\n");
        }
    }
    if !exporter.raw.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        if exporter.namespace.is_empty() {
            append_raw(&mut out, exporter);
        } else {
            out.push_str("namespace ");
            out.push_str(exporter.namespace.as_ref());
            out.push_str("\n{\n");
            for raw in &exporter.raw {
                for line in raw.trim_end().lines() {
                    out.push_str(exporter.indent.as_ref());
                    out.push_str(line);
                    out.push('\n');
                }
            }
            out.push_str("}\n");
        }
    }
    Ok(out)
}

fn render_file(
    exporter: &CSharp,
    types: &Types,
    ndts: &[&NamedDataType],
    namespace: &str,
    include_raw: bool,
) -> Result<String, Error> {
    let mut out = file_header(exporter);
    if !namespace.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("namespace ");
        out.push_str(namespace);
        out.push_str(";\n");
    }
    for (index, ndt) in ndts.iter().enumerate() {
        if index != 0 || !out.is_empty() {
            out.push('\n');
        }
        render_named(&mut out, exporter, types, ndt, "")?;
    }
    if include_raw {
        append_raw(&mut out, exporter);
    }
    Ok(out)
}

fn file_header(exporter: &CSharp) -> String {
    let mut out = String::from(
        "// This file has been generated by Specta. Do not edit this file manually.\n",
    );
    if !exporter.header.is_empty() {
        out.push_str(exporter.header.trim_end());
        out.push('\n');
    }
    out.push_str("#nullable enable\n");
    out
}

fn append_raw(out: &mut String, exporter: &CSharp) {
    for raw in &exporter.raw {
        out.push('\n');
        out.push_str(raw.trim_end());
        out.push('\n');
    }
}

fn render_named(
    out: &mut String,
    exporter: &CSharp,
    types: &Types,
    ndt: &NamedDataType,
    base: &str,
) -> Result<(), Error> {
    let Some(ty) = ndt.ty.as_ref() else {
        return Ok(());
    };
    if matches!(ty, DataType::Struct(strct) if is_non_object_struct(&strct.fields)) {
        return Ok(());
    }
    xml_docs(out, base, &ndt.docs);
    obsolete(out, base, ndt.deprecated.as_ref());
    let name = exported_name(exporter, ndt);
    let path = rust_path(ndt);
    let generics = generic_declarations(&ndt.generics, &name, &path)?;
    match ty {
        DataType::Struct(strct) => render_record(
            out,
            exporter,
            types,
            base,
            exporter.visibility.keyword(),
            &name,
            &generics,
            &strct.fields,
            &path,
        ),
        DataType::Enum(enm)
            if enm
                .variants
                .iter()
                .filter(|(_, v)| !v.skip)
                .all(|(_, v)| matches!(v.fields, Fields::Unit)) =>
        {
            render_simple_enum(out, exporter, base, &name, &generics, enm, &path)
        }
        DataType::Enum(enm) => {
            render_union(out, exporter, types, base, &name, &generics, enm, &path)
        }
        _ => {
            out.push_str(base);
            out.push_str(exporter.visibility.keyword());
            out.push_str(" sealed record ");
            out.push_str(&name);
            out.push_str(&generics);
            out.push('\n');
            out.push_str(base);
            out.push_str("{\n");
            let indent = format!("{base}{}", exporter.indent);
            out.push_str(&indent);
            out.push_str("public required ");
            datatype(out, exporter, types, ty, &format!("{path}.Value"))?;
            out.push_str(" Value { get; init; }\n");
            out.push_str(base);
            out.push_str("}\n");
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_record(
    out: &mut String,
    exporter: &CSharp,
    types: &Types,
    base: &str,
    visibility: &str,
    name: &str,
    generics: &str,
    fields: &Fields,
    path: &str,
) -> Result<(), Error> {
    out.push_str(base);
    out.push_str(visibility);
    out.push_str(" sealed record ");
    out.push_str(name);
    out.push_str(generics);
    out.push('\n');
    out.push_str(base);
    out.push_str("{\n");
    render_fields(out, exporter, types, fields, base, path, name)?;
    out.push_str(base);
    out.push_str("}\n");
    Ok(())
}

fn render_fields(
    out: &mut String,
    exporter: &CSharp,
    types: &Types,
    fields: &Fields,
    base: &str,
    path: &str,
    containing_name: &str,
) -> Result<(), Error> {
    let indent = format!("{base}{}", exporter.indent);
    let reserved_type_names = match exporter.layout {
        Layout::FlatFile | Layout::ModulePrefixedName => types
            .into_unsorted_iter()
            .map(|ndt| exported_name(exporter, ndt))
            .collect(),
        Layout::Namespaces | Layout::Files => HashSet::new(),
    };
    match fields {
        Fields::Unit => {}
        Fields::Named(fields) => {
            let mut used = record_reserved_names(containing_name);
            for (wire_name, field) in &fields.fields {
                let Some(ty) = field.ty.as_ref() else {
                    continue;
                };
                let mut property = property_name(wire_name);
                if !used.insert(property.clone()) {
                    let base = property.clone();
                    let mut suffix = 2;
                    loop {
                        property = format!("{base}{suffix}");
                        if used.insert(property.clone()) {
                            break;
                        }
                        suffix += 1;
                    }
                }
                let nested_name = unique_type_identifier(
                    format!("{property}Value"),
                    &mut used,
                    &reserved_type_names,
                );
                render_field_property(
                    out,
                    exporter,
                    types,
                    &indent,
                    &property,
                    &nested_name,
                    wire_name,
                    field,
                    ty,
                    path,
                    &mut used,
                    &reserved_type_names,
                )?;
            }
        }
        Fields::Unnamed(fields) => {
            let mut used = record_reserved_names(containing_name);
            for (index, field) in fields.fields.iter().enumerate() {
                let Some(ty) = field.ty.as_ref() else {
                    continue;
                };
                let name = unique_identifier(format!("Item{}", index + 1), &mut used);
                let nested_name =
                    unique_type_identifier(format!("{name}Value"), &mut used, &reserved_type_names);
                render_field_property(
                    out,
                    exporter,
                    types,
                    &indent,
                    &name,
                    &nested_name,
                    &index.to_string(),
                    field,
                    ty,
                    path,
                    &mut used,
                    &reserved_type_names,
                )?;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_field_property(
    out: &mut String,
    exporter: &CSharp,
    types: &Types,
    indent: &str,
    property: &str,
    nested_name: &str,
    wire_name: &str,
    field: &Field,
    ty: &DataType,
    path: &str,
    used: &mut HashSet<String>,
    reserved_type_names: &HashSet<String>,
) -> Result<(), Error> {
    if contains_recursive_inline(ty) {
        return Err(Error::RecursiveInline {
            path: format!("{path}.{wire_name}"),
        });
    }
    let mut structural_types = Vec::new();
    for structural_ty in collect_inline_structural_types(types, ty) {
        if structural_types
            .iter()
            .all(|(existing, _)| *existing != structural_ty)
        {
            let name = if structural_types.is_empty() {
                nested_name.to_string()
            } else {
                unique_type_identifier(
                    format!("{nested_name}{}", structural_types.len() + 1),
                    used,
                    reserved_type_names,
                )
            };
            structural_types.push((structural_ty, name));
        }
    }
    for (structural_ty, name) in &structural_types {
        match structural_ty {
            DataType::Struct(strct) => render_record(
                out,
                exporter,
                types,
                indent,
                "public",
                name,
                "",
                &strct.fields,
                &format!("{path}.{wire_name}"),
            )?,
            DataType::Enum(enm)
                if enm
                    .variants
                    .iter()
                    .filter(|(_, variant)| !variant.skip)
                    .all(|(_, variant)| matches!(variant.fields, Fields::Unit)) =>
            {
                render_simple_enum(out, exporter, indent, name, "", enm, path)?;
            }
            DataType::Enum(enm) => {
                render_union(out, exporter, types, indent, name, "", enm, path)?;
            }
            DataType::Intersection(_) => {
                return Err(Error::UnsupportedType {
                    path: format!("{path}.{wire_name}"),
                    kind: "intersection",
                });
            }
            _ => unreachable!("only structural datatypes are collected"),
        }
    }
    let type_override = (!structural_types.is_empty())
        .then(|| {
            render_datatype_with_inline_overrides(
                exporter,
                types,
                ty,
                &structural_types,
                &format!("{path}.{wire_name}"),
            )
        })
        .transpose()?;

    render_property(
        out,
        exporter,
        types,
        indent,
        property,
        wire_name,
        field,
        ty,
        type_override.as_deref(),
        path,
    )
}

fn collect_inline_structural_types<'a>(types: &'a Types, ty: &'a DataType) -> Vec<&'a DataType> {
    fn collect<'a>(
        types: &'a Types,
        ty: &'a DataType,
        generic_layers: &[GenericArguments<'a>],
        visiting: &mut HashSet<NamedReference>,
        found: &mut Vec<&'a DataType>,
    ) {
        match ty {
            DataType::Struct(strct) if is_non_object_struct(&strct.fields) => {
                if let Fields::Unnamed(fields) = &strct.fields {
                    for field in &fields.fields {
                        if let Some(ty) = &field.ty {
                            collect(types, ty, generic_layers, visiting, found);
                        }
                    }
                }
            }
            DataType::Struct(_) | DataType::Enum(_) | DataType::Intersection(_) => {
                found.push(ty);
            }
            DataType::List(list) => {
                collect(types, &list.ty, generic_layers, visiting, found);
            }
            DataType::Map(map) => {
                collect(types, map.key_ty(), generic_layers, visiting, found);
                collect(types, map.value_ty(), generic_layers, visiting, found);
            }
            DataType::Nullable(inner) => collect(types, inner, generic_layers, visiting, found),
            DataType::Tuple(tuple) => {
                for element in &tuple.elements {
                    collect(types, element, generic_layers, visiting, found);
                }
            }
            DataType::Reference(Reference::Named(reference)) => match &reference.inner {
                NamedReferenceType::Inline { dt, .. } => {
                    collect(types, dt, generic_layers, visiting, found);
                }
                NamedReferenceType::Reference { generics, .. } => {
                    for (_, generic) in generics {
                        collect(types, generic, generic_layers, visiting, found);
                    }
                    if visiting.insert(reference.clone()) {
                        if let Some(DataType::Struct(strct)) =
                            types.get(reference).and_then(|ndt| ndt.ty.as_ref())
                            && is_non_object_struct(&strct.fields)
                        {
                            let mut layers = generic_layers.to_vec();
                            layers.push(generics);
                            if let Fields::Unnamed(fields) = &strct.fields {
                                for field in &fields.fields {
                                    if let Some(ty) = &field.ty {
                                        collect(types, ty, &layers, visiting, found);
                                    }
                                }
                            }
                        }
                        visiting.remove(reference);
                    }
                }
                NamedReferenceType::Recursive(_) => {}
            },
            DataType::Generic(generic) => {
                for (layer_index, layer) in generic_layers.iter().enumerate().rev() {
                    if let Some((_, value)) =
                        layer.iter().find(|(candidate, _)| candidate == generic)
                    {
                        collect(
                            types,
                            value,
                            &generic_layers[..layer_index],
                            visiting,
                            found,
                        );
                        break;
                    }
                }
            }
            DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => {}
        }
    }

    let mut found = Vec::new();
    collect(types, ty, &[], &mut HashSet::new(), &mut found);
    found
}

fn render_datatype_with_inline_overrides(
    exporter: &CSharp,
    types: &Types,
    ty: &DataType,
    structural_types: &[(&DataType, String)],
    path: &str,
) -> Result<String, Error> {
    fn render(
        out: &mut String,
        exporter: &CSharp,
        types: &Types,
        ty: &DataType,
        structural_types: &[(&DataType, String)],
        path: &str,
        generic_layers: &[GenericArguments<'_>],
    ) -> Result<(), Error> {
        match ty {
            DataType::Struct(strct) if is_non_object_struct(&strct.fields) => {
                let fields = match &strct.fields {
                    Fields::Unit => Vec::new(),
                    Fields::Unnamed(fields) => fields
                        .fields
                        .iter()
                        .filter_map(|field| field.ty.as_ref())
                        .collect(),
                    Fields::Named(_) => unreachable!(),
                };
                match fields.as_slice() {
                    [] => out.push_str("object?"),
                    [field] => {
                        render(
                            out,
                            exporter,
                            types,
                            field,
                            structural_types,
                            path,
                            generic_layers,
                        )?;
                    }
                    fields => {
                        out.push('(');
                        for (index, field) in fields.iter().enumerate() {
                            if index != 0 {
                                out.push_str(", ");
                            }
                            render(
                                out,
                                exporter,
                                types,
                                field,
                                structural_types,
                                path,
                                generic_layers,
                            )?;
                        }
                        out.push(')');
                    }
                }
            }
            DataType::Struct(_) | DataType::Enum(_) | DataType::Intersection(_) => {
                let (_, name) = structural_types
                    .iter()
                    .find(|(structural_ty, _)| *structural_ty == ty)
                    .expect("all structural datatypes must have a nested name");
                out.push_str(name);
            }
            DataType::Reference(Reference::Named(reference)) => match &reference.inner {
                NamedReferenceType::Inline { dt, .. } => {
                    render(
                        out,
                        exporter,
                        types,
                        dt,
                        structural_types,
                        path,
                        generic_layers,
                    )?;
                }
                NamedReferenceType::Recursive(_) => {
                    return Err(Error::RecursiveInline { path: path.into() });
                }
                NamedReferenceType::Reference { generics, .. } => {
                    let ndt = types
                        .get(reference)
                        .ok_or_else(|| Error::DanglingReference { path: path.into() })?;
                    if let Some(DataType::Struct(strct)) = ndt.ty.as_ref()
                        && is_non_object_struct(&strct.fields)
                    {
                        let mut layers = generic_layers.to_vec();
                        layers.push(generics);
                        let fields = match &strct.fields {
                            Fields::Unit => Vec::new(),
                            Fields::Unnamed(fields) => fields
                                .fields
                                .iter()
                                .filter_map(|field| field.ty.as_ref())
                                .collect(),
                            Fields::Named(_) => unreachable!(),
                        };
                        match fields.as_slice() {
                            [] => out.push_str("object?"),
                            [field] => render(
                                out,
                                exporter,
                                types,
                                field,
                                structural_types,
                                path,
                                &layers,
                            )?,
                            fields => {
                                out.push('(');
                                for (index, field) in fields.iter().enumerate() {
                                    if index != 0 {
                                        out.push_str(", ");
                                    }
                                    render(
                                        out,
                                        exporter,
                                        types,
                                        field,
                                        structural_types,
                                        path,
                                        &layers,
                                    )?;
                                }
                                out.push(')');
                            }
                        }
                    } else {
                        reference_name(out, exporter, ndt);
                        if !generics.is_empty() {
                            out.push('<');
                            for (index, (_, generic)) in generics.iter().enumerate() {
                                if index != 0 {
                                    out.push_str(", ");
                                }
                                render(
                                    out,
                                    exporter,
                                    types,
                                    generic,
                                    structural_types,
                                    path,
                                    generic_layers,
                                )?;
                            }
                            out.push('>');
                        }
                    }
                }
            },
            DataType::Nullable(inner) => {
                let mut rendered = String::new();
                render(
                    &mut rendered,
                    exporter,
                    types,
                    inner,
                    structural_types,
                    path,
                    generic_layers,
                )?;
                out.push_str(&rendered);
                if !rendered.ends_with('?') {
                    out.push('?');
                }
            }
            DataType::List(list) => {
                out.push_str("global::System.Collections.Generic.IReadOnlyList<");
                render(
                    out,
                    exporter,
                    types,
                    &list.ty,
                    structural_types,
                    path,
                    generic_layers,
                )?;
                out.push('>');
            }
            DataType::Map(map) => {
                out.push_str("global::System.Collections.Generic.IReadOnlyDictionary<");
                render(
                    out,
                    exporter,
                    types,
                    map.key_ty(),
                    structural_types,
                    path,
                    generic_layers,
                )?;
                out.push_str(", ");
                render(
                    out,
                    exporter,
                    types,
                    map.value_ty(),
                    structural_types,
                    path,
                    generic_layers,
                )?;
                out.push('>');
            }
            DataType::Tuple(tuple) => match tuple.elements.as_slice() {
                [] => out.push_str("global::System.ValueTuple"),
                [element] => {
                    out.push_str("global::System.ValueTuple<");
                    render(
                        out,
                        exporter,
                        types,
                        element,
                        structural_types,
                        path,
                        generic_layers,
                    )?;
                    out.push('>');
                }
                elements => {
                    out.push('(');
                    for (index, element) in elements.iter().enumerate() {
                        if index != 0 {
                            out.push_str(", ");
                        }
                        render(
                            out,
                            exporter,
                            types,
                            element,
                            structural_types,
                            path,
                            generic_layers,
                        )?;
                    }
                    out.push(')');
                }
            },
            DataType::Generic(generic) => {
                for (layer_index, layer) in generic_layers.iter().enumerate().rev() {
                    if let Some((_, value)) =
                        layer.iter().find(|(candidate, _)| candidate == generic)
                    {
                        return render(
                            out,
                            exporter,
                            types,
                            value,
                            structural_types,
                            path,
                            &generic_layers[..layer_index],
                        );
                    }
                }
                datatype(out, exporter, types, ty, path)?;
            }
            _ => datatype(out, exporter, types, ty, path)?,
        }
        Ok(())
    }

    let mut out = String::new();
    render(&mut out, exporter, types, ty, structural_types, path, &[])?;
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn render_property(
    out: &mut String,
    exporter: &CSharp,
    types: &Types,
    indent: &str,
    property: &str,
    wire_name: &str,
    field: &Field,
    ty: &DataType,
    type_override: Option<&str>,
    path: &str,
) -> Result<(), Error> {
    xml_docs(out, indent, &field.docs);
    obsolete(out, indent, field.deprecated.as_ref());
    if property.trim_start_matches('@') != wire_name {
        out.push_str(indent);
        out.push_str("[global::System.Text.Json.Serialization.JsonPropertyName(\"");
        out.push_str(&escape_csharp_string(wire_name));
        out.push_str("\")]\n");
    }
    out.push_str(indent);
    out.push_str("public ");
    if !field.optional {
        out.push_str("required ");
    }
    let rendered_type = if let Some(type_override) = type_override {
        type_override.to_string()
    } else {
        let mut rendered = String::new();
        datatype(
            &mut rendered,
            exporter,
            types,
            ty,
            &format!("{path}.{wire_name}"),
        )?;
        rendered
    };
    out.push_str(&rendered_type);
    if field.optional && !rendered_type.ends_with('?') {
        out.push('?');
    }
    out.push(' ');
    out.push_str(property);
    out.push_str(" { get; init; }\n");
    Ok(())
}

fn render_simple_enum(
    out: &mut String,
    exporter: &CSharp,
    base: &str,
    name: &str,
    generics: &str,
    enm: &specta::datatype::Enum,
    path: &str,
) -> Result<(), Error> {
    if !generics.is_empty() {
        return render_union(
            out,
            exporter,
            &Types::default(),
            base,
            name,
            generics,
            enm,
            path,
        );
    }
    out.push_str(base);
    out.push_str(exporter.visibility.keyword());
    out.push_str(" enum ");
    out.push_str(name);
    out.push('\n');
    out.push_str(base);
    out.push_str("{\n");
    let indent = format!("{base}{}", exporter.indent);
    let mut used = HashSet::from([name.to_string()]);
    for (wire_name, variant) in enm.variants.iter().filter(|(_, variant)| !variant.skip) {
        xml_docs(out, &indent, &variant.docs);
        obsolete(out, &indent, variant.deprecated.as_ref());
        let member = unique_identifier(property_name(wire_name), &mut used);
        out.push_str(&indent);
        out.push_str(&member);
        out.push_str(",\n");
    }
    out.push_str(base);
    out.push_str("}\n");
    Ok(())
}

fn render_union(
    out: &mut String,
    exporter: &CSharp,
    types: &Types,
    base: &str,
    name: &str,
    generics: &str,
    enm: &specta::datatype::Enum,
    path: &str,
) -> Result<(), Error> {
    out.push_str(base);
    out.push_str(exporter.visibility.keyword());
    out.push_str(" abstract record ");
    out.push_str(name);
    out.push_str(generics);
    out.push('\n');
    out.push_str(base);
    out.push_str("{\n");
    let indent = format!("{base}{}", exporter.indent);
    let mut used = record_reserved_names(name);
    for (wire_name, variant) in enm.variants.iter().filter(|(_, variant)| !variant.skip) {
        let variant_name = unique_identifier(property_name(wire_name), &mut used);
        render_variant(
            out,
            exporter,
            types,
            &indent,
            name,
            generics,
            &variant_name,
            wire_name,
            variant,
            path,
        )?;
    }
    out.push_str(base);
    out.push_str("}\n");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_variant(
    out: &mut String,
    exporter: &CSharp,
    types: &Types,
    indent: &str,
    parent: &str,
    generics: &str,
    variant_name: &str,
    wire_name: &str,
    variant: &Variant,
    path: &str,
) -> Result<(), Error> {
    xml_docs(out, indent, &variant.docs);
    obsolete(out, indent, variant.deprecated.as_ref());
    out.push_str(indent);
    out.push_str("public sealed record ");
    out.push_str(variant_name);
    out.push_str(" : ");
    out.push_str(parent);
    out.push_str(generics);
    out.push('\n');
    out.push_str(indent);
    out.push_str("{\n");
    render_fields(
        out,
        exporter,
        types,
        &variant.fields,
        indent,
        &format!("{path}.{wire_name}"),
        variant_name,
    )?;
    out.push_str(indent);
    out.push_str("}\n");
    Ok(())
}

fn datatype(
    out: &mut String,
    exporter: &CSharp,
    types: &Types,
    ty: &DataType,
    path: &str,
) -> Result<(), Error> {
    if contains_recursive_inline(ty) {
        return Err(Error::RecursiveInline { path: path.into() });
    }
    match ty {
        DataType::Primitive(Primitive::f128) => {
            return Err(Error::UnsupportedType {
                path: path.into(),
                kind: "128-bit floating-point",
            });
        }
        DataType::Primitive(primitive) => out.push_str(primitive_name(primitive)),
        DataType::List(list) => {
            out.push_str("global::System.Collections.Generic.IReadOnlyList<");
            datatype(out, exporter, types, &list.ty, path)?;
            out.push('>');
        }
        DataType::Map(map) => {
            out.push_str("global::System.Collections.Generic.IReadOnlyDictionary<");
            datatype(out, exporter, types, map.key_ty(), path)?;
            out.push_str(", ");
            datatype(out, exporter, types, map.value_ty(), path)?;
            out.push('>');
        }
        DataType::Nullable(inner) => {
            let mut rendered = String::new();
            datatype(&mut rendered, exporter, types, inner, path)?;
            out.push_str(&rendered);
            if !rendered.ends_with('?') {
                out.push('?');
            }
        }
        DataType::Tuple(tuple) => match tuple.elements.as_slice() {
            [] => out.push_str("global::System.ValueTuple"),
            [element] => {
                out.push_str("global::System.ValueTuple<");
                datatype(out, exporter, types, element, path)?;
                out.push('>');
            }
            elements => {
                out.push('(');
                for (index, element) in elements.iter().enumerate() {
                    if index != 0 {
                        out.push_str(", ");
                    }
                    datatype(out, exporter, types, element, path)?;
                }
                out.push(')');
            }
        },
        DataType::Generic(generic) => out.push_str(&identifier(generic.name(), path)?),
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => datatype(out, exporter, types, dt, path)?,
            NamedReferenceType::Recursive(_) => {
                return Err(Error::RecursiveInline { path: path.into() });
            }
            NamedReferenceType::Reference { .. } => {
                let ndt = types
                    .get(reference)
                    .ok_or_else(|| Error::DanglingReference { path: path.into() })?;
                if let Some(DataType::Struct(strct)) = ndt.ty.as_ref()
                    && is_non_object_struct(&strct.fields)
                {
                    let NamedReferenceType::Reference { generics, .. } = &reference.inner else {
                        unreachable!()
                    };
                    return render_non_object_struct(
                        out,
                        exporter,
                        types,
                        &strct.fields,
                        path,
                        &[generics],
                    );
                }
                reference_name(out, exporter, ndt);
                if let NamedReferenceType::Reference { generics, .. } = &reference.inner
                    && !generics.is_empty()
                {
                    out.push('<');
                    for (index, (_, generic)) in generics.iter().enumerate() {
                        if index != 0 {
                            out.push_str(", ");
                        }
                        datatype(out, exporter, types, generic, path)?;
                    }
                    out.push('>');
                }
            }
        },
        DataType::Reference(Reference::Opaque(reference)) => {
            let name = reference.type_name();
            if let Some(mapped) = exporter.opaque_types.get(name) {
                out.push_str(mapped);
            } else {
                out.push_str(opaque_name(name).ok_or_else(|| Error::UnsupportedOpaque {
                    path: path.into(),
                    name: name.into(),
                })?);
            }
        }
        DataType::Struct(strct) if is_non_object_struct(&strct.fields) => {
            render_non_object_struct(out, exporter, types, &strct.fields, path, &[])?;
        }
        DataType::Struct(_) => {
            return Err(Error::UnsupportedType {
                path: path.into(),
                kind: "struct",
            });
        }
        DataType::Enum(_) => {
            return Err(Error::UnsupportedType {
                path: path.into(),
                kind: "enum",
            });
        }
        DataType::Intersection(_) => {
            return Err(Error::UnsupportedType {
                path: path.into(),
                kind: "intersection",
            });
        }
    }
    Ok(())
}

type GenericArguments<'a> = &'a [(specta::datatype::Generic, DataType)];

fn is_non_object_struct(fields: &Fields) -> bool {
    matches!(fields, Fields::Unit | Fields::Unnamed(_))
}

fn is_emitted_named(ndt: &NamedDataType) -> bool {
    ndt.ty.as_ref().is_some_and(
        |ty| !matches!(ty, DataType::Struct(strct) if is_non_object_struct(&strct.fields)),
    )
}

fn render_non_object_struct(
    out: &mut String,
    exporter: &CSharp,
    types: &Types,
    fields: &Fields,
    path: &str,
    generic_layers: &[GenericArguments<'_>],
) -> Result<(), Error> {
    let fields = match fields {
        Fields::Unit => Vec::new(),
        Fields::Unnamed(fields) => fields
            .fields
            .iter()
            .filter_map(|field| field.ty.as_ref())
            .collect(),
        Fields::Named(_) => unreachable!("named fields have an object wire shape"),
    };
    match fields.as_slice() {
        [] => out.push_str("object?"),
        [field] => wire_datatype(out, exporter, types, field, path, generic_layers)?,
        fields => {
            out.push('(');
            for (index, field) in fields.iter().enumerate() {
                if index != 0 {
                    out.push_str(", ");
                }
                wire_datatype(out, exporter, types, field, path, generic_layers)?;
            }
            out.push(')');
        }
    }
    Ok(())
}

fn wire_datatype(
    out: &mut String,
    exporter: &CSharp,
    types: &Types,
    ty: &DataType,
    path: &str,
    generic_layers: &[GenericArguments<'_>],
) -> Result<(), Error> {
    if contains_recursive_inline(ty) {
        return Err(Error::RecursiveInline { path: path.into() });
    }
    match ty {
        DataType::Generic(generic) => {
            for (layer_index, layer) in generic_layers.iter().enumerate().rev() {
                if let Some((_, value)) = layer.iter().find(|(candidate, _)| candidate == generic) {
                    return wire_datatype(
                        out,
                        exporter,
                        types,
                        value,
                        path,
                        &generic_layers[..layer_index],
                    );
                }
            }
            datatype(out, exporter, types, ty, path)?;
        }
        DataType::List(list) => {
            out.push_str("global::System.Collections.Generic.IReadOnlyList<");
            wire_datatype(out, exporter, types, &list.ty, path, generic_layers)?;
            out.push('>');
        }
        DataType::Map(map) => {
            out.push_str("global::System.Collections.Generic.IReadOnlyDictionary<");
            wire_datatype(out, exporter, types, map.key_ty(), path, generic_layers)?;
            out.push_str(", ");
            wire_datatype(out, exporter, types, map.value_ty(), path, generic_layers)?;
            out.push('>');
        }
        DataType::Nullable(inner) => {
            let mut rendered = String::new();
            wire_datatype(&mut rendered, exporter, types, inner, path, generic_layers)?;
            out.push_str(&rendered);
            if !rendered.ends_with('?') {
                out.push('?');
            }
        }
        DataType::Tuple(tuple) => match tuple.elements.as_slice() {
            [] => out.push_str("global::System.ValueTuple"),
            [element] => {
                out.push_str("global::System.ValueTuple<");
                wire_datatype(out, exporter, types, element, path, generic_layers)?;
                out.push('>');
            }
            elements => {
                out.push('(');
                for (index, element) in elements.iter().enumerate() {
                    if index != 0 {
                        out.push_str(", ");
                    }
                    wire_datatype(out, exporter, types, element, path, generic_layers)?;
                }
                out.push(')');
            }
        },
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => {
                wire_datatype(out, exporter, types, dt, path, generic_layers)?;
            }
            NamedReferenceType::Recursive(_) => {
                return Err(Error::RecursiveInline { path: path.into() });
            }
            NamedReferenceType::Reference { generics, .. } => {
                let ndt = types
                    .get(reference)
                    .ok_or_else(|| Error::DanglingReference { path: path.into() })?;
                if let Some(DataType::Struct(strct)) = ndt.ty.as_ref()
                    && is_non_object_struct(&strct.fields)
                {
                    let mut layers = generic_layers.to_vec();
                    layers.push(generics);
                    render_non_object_struct(out, exporter, types, &strct.fields, path, &layers)?;
                } else {
                    reference_name(out, exporter, ndt);
                    if !generics.is_empty() {
                        out.push('<');
                        for (index, (_, generic)) in generics.iter().enumerate() {
                            if index != 0 {
                                out.push_str(", ");
                            }
                            wire_datatype(out, exporter, types, generic, path, generic_layers)?;
                        }
                        out.push('>');
                    }
                }
            }
        },
        DataType::Struct(strct) if is_non_object_struct(&strct.fields) => {
            render_non_object_struct(out, exporter, types, &strct.fields, path, generic_layers)?;
        }
        _ => datatype(out, exporter, types, ty, path)?,
    }
    Ok(())
}

fn contains_recursive_inline(ty: &DataType) -> bool {
    match ty {
        DataType::Primitive(_)
        | DataType::Generic(_)
        | DataType::Reference(Reference::Opaque(_)) => false,
        DataType::List(list) => contains_recursive_inline(&list.ty),
        DataType::Map(map) => {
            contains_recursive_inline(map.key_ty()) || contains_recursive_inline(map.value_ty())
        }
        DataType::Nullable(inner) => contains_recursive_inline(inner),
        DataType::Tuple(tuple) => tuple.elements.iter().any(contains_recursive_inline),
        DataType::Intersection(types) => types.iter().any(contains_recursive_inline),
        DataType::Struct(strct) => fields_contain_recursive_inline(&strct.fields),
        DataType::Enum(enm) => enm
            .variants
            .iter()
            .any(|(_, variant)| fields_contain_recursive_inline(&variant.fields)),
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Recursive(_) => true,
            NamedReferenceType::Inline { dt, .. } => contains_recursive_inline(dt),
            NamedReferenceType::Reference { generics, .. } => generics
                .iter()
                .any(|(_, datatype)| contains_recursive_inline(datatype)),
        },
    }
}

fn fields_contain_recursive_inline(fields: &Fields) -> bool {
    match fields {
        Fields::Unit => false,
        Fields::Unnamed(fields) => fields
            .fields
            .iter()
            .filter_map(|field| field.ty.as_ref())
            .any(contains_recursive_inline),
        Fields::Named(fields) => fields
            .fields
            .iter()
            .filter_map(|(_, field)| field.ty.as_ref())
            .any(contains_recursive_inline),
    }
}

fn primitive_name(primitive: &Primitive) -> &'static str {
    match primitive {
        Primitive::i8 => "sbyte",
        Primitive::i16 => "short",
        Primitive::i32 => "int",
        Primitive::i64 => "long",
        Primitive::i128 => "global::System.Numerics.BigInteger",
        Primitive::isize => "nint",
        Primitive::u8 => "byte",
        Primitive::u16 => "ushort",
        Primitive::u32 => "uint",
        Primitive::u64 => "ulong",
        Primitive::u128 => "global::System.Numerics.BigInteger",
        Primitive::usize => "nuint",
        Primitive::f16 => "global::System.Half",
        Primitive::f32 => "float",
        Primitive::f64 => "double",
        Primitive::f128 => unreachable!("f128 is rejected before primitive rendering"),
        Primitive::bool => "bool",
        Primitive::char => "string",
        Primitive::str => "string",
    }
}

fn opaque_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "String" | "str" => "string",
        "char" => "char",
        "bool" => "bool",
        "i8" => "sbyte",
        "i16" => "short",
        "i32" => "int",
        "i64" => "long",
        "u8" => "byte",
        "u16" => "ushort",
        "u32" => "uint",
        "u64" => "ulong",
        "f32" => "float",
        "f64" => "double",
        "Uuid" | "UUID" => "global::System.Guid",
        "Duration" => "global::System.TimeSpan",
        "SystemTime" | "DateTime" | "NaiveDateTime" => "global::System.DateTimeOffset",
        _ => return None,
    })
}

fn reference_name(out: &mut String, exporter: &CSharp, ndt: &NamedDataType) {
    match exporter.layout {
        Layout::Namespaces | Layout::Files => {
            let namespace = joined_namespace(
                exporter.namespace.as_ref(),
                &module_segments(&ndt.module_path),
            );
            if !namespace.is_empty() {
                out.push_str("global::");
                out.push_str(&namespace);
                out.push('.');
            }
            out.push_str(&exported_name(exporter, ndt));
        }
        Layout::FlatFile | Layout::ModulePrefixedName => {
            out.push_str(&exported_name(exporter, ndt))
        }
    }
}

fn format_types<'a>(types: &'a Types, format: &dyn Format) -> Result<Cow<'a, Types>, Error> {
    let mapped = format
        .map_types(types)
        .map_err(|err| Error::format("type graph formatter failed", err))?;
    let source = mapped.as_ref();
    let mut mapped_types = source.clone();
    let mut failure = None;
    mapped_types.iter_mut(|ndt| {
        if failure.is_some() {
            return;
        }
        let Some(ty) = ndt.ty.as_ref() else {
            return;
        };
        let mut ty = ty.clone();
        match map_children(format, source, &mut ty, &rust_path(ndt)) {
            Ok(()) => ndt.ty = Some(ty),
            Err(err) => failure = Some(err),
        }
    });
    failure.map_or_else(|| Ok(Cow::Owned(mapped_types)), Err)
}

fn map_datatype(
    format: &dyn Format,
    types: &Types,
    ty: &DataType,
    path: &str,
) -> Result<DataType, Error> {
    if matches!(ty, DataType::Generic(_)) {
        return Ok(ty.clone());
    }
    let mut ty = format
        .map_type(types, ty)
        .map_err(|err| Error::format_at("datatype formatter failed", path, err))?
        .into_owned();
    map_children(format, types, &mut ty, path)?;
    Ok(ty)
}

fn map_children(
    format: &dyn Format,
    types: &Types,
    ty: &mut DataType,
    path: &str,
) -> Result<(), Error> {
    match ty {
        DataType::Primitive(_)
        | DataType::Generic(_)
        | DataType::Reference(Reference::Opaque(_)) => {}
        DataType::List(list) => {
            *list.ty = map_datatype(format, types, &list.ty, &format!("{path}.<list_item>"))?
        }
        DataType::Map(map) => {
            map.set_key_ty(map_datatype(
                format,
                types,
                map.key_ty(),
                &format!("{path}.<map_key>"),
            )?);
            map.set_value_ty(map_datatype(
                format,
                types,
                map.value_ty(),
                &format!("{path}.<map_value>"),
            )?);
        }
        DataType::Nullable(inner) => **inner = map_datatype(format, types, inner, path)?,
        DataType::Tuple(tuple) => {
            for (index, ty) in tuple.elements.iter_mut().enumerate() {
                *ty = map_datatype(format, types, ty, &format!("{path}.{index}"))?;
            }
        }
        DataType::Intersection(types_) => {
            for (index, ty) in types_.iter_mut().enumerate() {
                *ty = map_datatype(format, types, ty, &format!("{path}.<intersection_{index}>"))?;
            }
        }
        DataType::Struct(strct) => map_fields(format, types, &mut strct.fields, path)?,
        DataType::Enum(enm) => {
            for (name, variant) in &mut enm.variants {
                map_fields(
                    format,
                    types,
                    &mut variant.fields,
                    &format!("{path}.{name}"),
                )?;
            }
        }
        DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
            NamedReferenceType::Inline { dt, .. } => **dt = map_datatype(format, types, dt, path)?,
            NamedReferenceType::Reference { generics, .. } => {
                for (generic, ty) in generics {
                    *ty = map_datatype(format, types, ty, &format!("{path}.<{}>", generic.name()))?;
                }
            }
            NamedReferenceType::Recursive(_) => {}
        },
    }
    Ok(())
}

fn map_fields(
    format: &dyn Format,
    types: &Types,
    fields: &mut Fields,
    path: &str,
) -> Result<(), Error> {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => {
            for (index, field) in fields.fields.iter_mut().enumerate() {
                if let Some(ty) = &mut field.ty {
                    *ty = map_datatype(format, types, ty, &format!("{path}.{index}"))?;
                }
            }
        }
        Fields::Named(fields) => {
            for (name, field) in &mut fields.fields {
                if let Some(ty) = &mut field.ty {
                    *ty = map_datatype(format, types, ty, &format!("{path}.{name}"))?;
                }
            }
        }
    }
    Ok(())
}

fn validate_names(exporter: &CSharp, types: &Types) -> Result<(), Error> {
    validate_namespace(exporter.namespace.as_ref())?;
    let ndts = types
        .into_sorted_iter()
        .filter(|ndt| is_emitted_named(ndt))
        .collect::<Vec<_>>();
    if matches!(exporter.layout, Layout::Namespaces | Layout::Files) {
        let mut namespaces = HashSet::new();
        for ndt in &ndts {
            let module = module_segments(&ndt.module_path);
            for length in 1..=module.len() {
                namespaces.insert(module[..length].to_vec());
            }
        }
        for ndt in &ndts {
            let mut declaration = module_segments(&ndt.module_path);
            declaration.push(exported_name(exporter, ndt));
            if namespaces.contains(&declaration) {
                return Err(Error::DuplicateTypeName {
                    name: declaration.join("."),
                    first: rust_path(ndt),
                    second: "C# namespace".into(),
                });
            }
        }
    }
    let mut scopes: HashMap<Vec<String>, HashMap<String, String>> = HashMap::new();
    for ndt in ndts {
        validate_module_path(&ndt.module_path)?;
        let name = exported_name(exporter, ndt);
        identifier(&name, &rust_path(ndt))?;
        let scope = match exporter.layout {
            Layout::Namespaces | Layout::Files => module_segments(&ndt.module_path),
            Layout::FlatFile | Layout::ModulePrefixedName => Vec::new(),
        };
        let path = rust_path(ndt);
        if let Some(first) = scopes
            .entry(scope)
            .or_default()
            .insert(name.clone(), path.clone())
        {
            return Err(Error::DuplicateTypeName {
                name,
                first,
                second: path,
            });
        }
    }
    Ok(())
}

fn validate_namespace(namespace: &str) -> Result<(), Error> {
    if namespace.is_empty() {
        return Ok(());
    }
    for segment in namespace.split('.') {
        let escaped = identifier(segment, "C# namespace")?;
        if escaped != segment {
            return Err(Error::InvalidName {
                path: "C# namespace".into(),
                name: namespace.into(),
            });
        }
    }
    Ok(())
}

fn validate_module_path(module: &str) -> Result<(), Error> {
    for segment in module.split("::").filter(|segment| !segment.is_empty()) {
        if matches!(segment, "." | "..")
            || segment.contains('/')
            || segment.contains('\\')
            || identifier(&pascal_case(segment), "Rust module path").is_err()
        {
            return Err(Error::InvalidName {
                path: "Rust module path".into(),
                name: module.into(),
            });
        }
    }
    Ok(())
}

fn exported_name(exporter: &CSharp, ndt: &NamedDataType) -> String {
    if exporter.layout == Layout::ModulePrefixedName {
        let mut segments = module_segments(&ndt.module_path);
        segments.push(pascal_case(&ndt.name));
        segments.join("_")
    } else {
        pascal_case(&ndt.name)
    }
}

fn generic_declarations(
    generics: &[specta::datatype::GenericDefinition],
    containing_name: &str,
    path: &str,
) -> Result<String, Error> {
    if generics.is_empty() {
        return Ok(String::new());
    }
    let names = generics
        .iter()
        .map(|generic| identifier(&generic.name, path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut unique = HashSet::new();
    if let Some(duplicate) = names.iter().find(|name| !unique.insert(name.as_str())) {
        return Err(Error::InvalidName {
            path: path.into(),
            name: duplicate.to_string(),
        });
    }
    if names.iter().any(|name| name == containing_name) {
        return Err(Error::InvalidName {
            path: path.into(),
            name: containing_name.into(),
        });
    }
    Ok(format!("<{}>", names.join(", ")))
}

fn identifier(name: &str, path: &str) -> Result<String, Error> {
    let raw = name.strip_prefix('@').unwrap_or(name);
    let mut chars = raw.chars();
    let valid = chars.next().is_some_and(|c| c == '_' || c.is_alphabetic())
        && chars.all(|c| c == '_' || c.is_alphanumeric());
    if !valid {
        return Err(Error::InvalidName {
            path: path.into(),
            name: name.into(),
        });
    }
    Ok(if is_keyword(raw) {
        format!("@{raw}")
    } else {
        raw.into()
    })
}

fn property_name(name: &str) -> String {
    let name = pascal_case(name);
    identifier(&name, "property").unwrap_or_else(|_| {
        let mut value = String::from("Field");
        for ch in name.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                value.push(ch);
            }
        }
        value
    })
}

fn unique_identifier(mut name: String, used: &mut HashSet<String>) -> String {
    if used.insert(name.clone()) {
        return name;
    }

    let base = name.clone();
    let mut suffix = 2;
    loop {
        name = format!("{base}{suffix}");
        if used.insert(name.clone()) {
            return name;
        }
        suffix += 1;
    }
}

fn unique_type_identifier(
    mut name: String,
    used: &mut HashSet<String>,
    reserved: &HashSet<String>,
) -> String {
    if !reserved.contains(&name) && used.insert(name.clone()) {
        return name;
    }

    let base = name.clone();
    let mut suffix = 2;
    loop {
        name = format!("{base}{suffix}");
        if !reserved.contains(&name) && used.insert(name.clone()) {
            return name;
        }
        suffix += 1;
    }
}

fn record_reserved_names(containing_name: &str) -> HashSet<String> {
    [
        containing_name,
        "Clone",
        "EqualityContract",
        "PrintMembers",
        "Equals",
        "GetHashCode",
        "ToString",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn pascal_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut uppercase = true;
    for ch in name.chars() {
        if ch == '_' || ch == '-' || ch == ' ' {
            uppercase = true;
        } else if uppercase {
            out.extend(ch.to_uppercase());
            uppercase = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn module_segments(module: &str) -> Vec<String> {
    module
        .split("::")
        .filter(|part| !part.is_empty() && *part != "virtual")
        .map(pascal_case)
        .collect()
}

fn joined_namespace(root: &str, module: &[String]) -> String {
    std::iter::once(root)
        .filter(|part| !part.is_empty())
        .chain(module.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(".")
}

fn rust_path(ndt: &NamedDataType) -> String {
    if ndt.module_path.is_empty() {
        ndt.name.to_string()
    } else {
        format!("{}::{}", ndt.module_path, ndt.name)
    }
}

fn xml_docs(out: &mut String, indent: &str, docs: &str) {
    for line in docs.lines() {
        out.push_str(indent);
        out.push_str("/// ");
        out.push_str(&escape_xml(line));
        out.push('\n');
    }
}

fn obsolete(out: &mut String, indent: &str, deprecated: Option<&Deprecated>) {
    let Some(deprecated) = deprecated else {
        return;
    };
    out.push_str(indent);
    out.push_str("[global::System.Obsolete");
    if let Some(note) = deprecated
        .note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty())
    {
        out.push_str("(\"");
        out.push_str(&escape_csharp_string(note));
        out.push_str("\")");
    }
    out.push_str("]\n");
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_csharp_string(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            ch => vec![ch],
        })
        .collect()
}

fn is_keyword(name: &str) -> bool {
    matches!(
        name,
        "abstract"
            | "as"
            | "base"
            | "bool"
            | "break"
            | "byte"
            | "case"
            | "catch"
            | "char"
            | "checked"
            | "class"
            | "const"
            | "continue"
            | "decimal"
            | "default"
            | "delegate"
            | "do"
            | "double"
            | "else"
            | "enum"
            | "event"
            | "explicit"
            | "extern"
            | "false"
            | "finally"
            | "fixed"
            | "float"
            | "for"
            | "foreach"
            | "goto"
            | "if"
            | "implicit"
            | "in"
            | "int"
            | "interface"
            | "internal"
            | "is"
            | "lock"
            | "long"
            | "namespace"
            | "new"
            | "null"
            | "object"
            | "operator"
            | "out"
            | "override"
            | "params"
            | "private"
            | "protected"
            | "public"
            | "readonly"
            | "record"
            | "ref"
            | "return"
            | "sbyte"
            | "sealed"
            | "short"
            | "sizeof"
            | "stackalloc"
            | "static"
            | "string"
            | "struct"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "uint"
            | "ulong"
            | "unchecked"
            | "unsafe"
            | "ushort"
            | "using"
            | "virtual"
            | "void"
            | "volatile"
            | "while"
    )
}

fn remove_stale_generated_files(root: &Path, expected: &[PathBuf]) -> Result<(), Error> {
    let expected = expected.iter().collect::<std::collections::HashSet<_>>();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|err| Error::io(&dir, err))? {
            let entry = entry.map_err(|err| Error::io(&dir, err))?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|err| Error::io(&path, err))?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                dirs.push(path);
            } else if file_type.is_file()
                && path.extension().is_some_and(|extension| extension == "cs")
                && !expected.contains(&path)
            {
                let content =
                    std::fs::read_to_string(&path).map_err(|err| Error::io(&path, err))?;
                if content
                    .lines()
                    .next()
                    .is_some_and(|line| line == "// This file has been generated by Specta. Do not edit this file manually.")
                {
                    std::fs::remove_file(&path).map_err(|err| Error::io(&path, err))?;
                }
            }
        }
    }
    Ok(())
}
