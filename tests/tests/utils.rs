use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
};

use specta::{
    Types,
    datatype::{DataType, Fields, Primitive},
};

/// Get a `String` representation of the filesystem.
/// This is used for snapshot testing multi-file exports.
pub fn fs_to_string(path: &Path) -> Result<String, std::io::Error> {
    let mut output = String::new();

    // Handle single file case
    if path.is_file() {
        let contents = fs::read(path)?;
        let name = path.file_name().unwrap().to_string_lossy();

        match String::from_utf8(contents) {
            Ok(text) => {
                let normalized = normalize_newlines(&text);
                output.push_str(&format!("{} ({} bytes)\n", name, normalized.len()));
                output.push_str("────────────────────────────────────────\n");

                for line in normalized.lines() {
                    output.push_str(&format!("{}\n", line));
                }
            }
            Err(err) => {
                output.push_str(&format!("{} ({} bytes)\n", name, err.as_bytes().len()));
                output.push_str("────────────────────────────────────────\n");
                output.push_str("[Binary file]\n");
            }
        }

        output.push_str("════════════════════════════════════════\n");
    } else {
        fs_to_string_impl(path, path, &mut output, "")?;
    }

    Ok(output)
}

fn fs_to_string_impl(
    root: &Path,
    current: &Path,
    output: &mut String,
    indent: &str,
) -> Result<(), std::io::Error> {
    let mut entries: Vec<PathBuf> = fs::read_dir(current)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect();

    // Sort entries: directories first, then files, both alphabetically
    entries.sort_by(|a, b| {
        let a_is_dir = a.is_dir();
        let b_is_dir = b.is_dir();

        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in entries {
        let name = entry.file_name().unwrap().to_string_lossy();

        if entry.is_dir() {
            output.push_str(&format!("{}{}/\n", indent, name));
            fs_to_string_impl(root, &entry, output, &format!("{}  ", indent))?;
        } else {
            let contents = fs::read(&entry)?;

            // Try to read as UTF-8, otherwise show as binary
            match String::from_utf8(contents) {
                Ok(text) => {
                    let normalized = normalize_newlines(&text);
                    output.push_str(&format!(
                        "{}{} ({} bytes)\n",
                        indent,
                        name,
                        normalized.len()
                    ));
                    output.push_str(&format!(
                        "{}────────────────────────────────────────\n",
                        indent
                    ));

                    for line in normalized.lines() {
                        output.push_str(&format!("{}{}\n", indent, line));
                    }
                }
                Err(err) => {
                    output.push_str(&format!(
                        "{}{} ({} bytes)\n",
                        indent,
                        name,
                        err.as_bytes().len()
                    ));
                    output.push_str(&format!(
                        "{}────────────────────────────────────────\n",
                        indent
                    ));
                    output.push_str(&format!("{}[Binary file]\n", indent));
                }
            }

            output.push_str(&format!(
                "{}════════════════════════════════════════\n\n",
                indent
            ));
        }
    }

    Ok(())
}

fn normalize_newlines(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

pub fn sanitize_typescript_bigints_in_dts(
    mut dts: Vec<(&'static str, DataType)>,
) -> Vec<(&'static str, DataType)> {
    for (_, dt) in &mut dts {
        sanitize_typescript_bigints_in_datatype(dt);
    }

    dts.retain(
        |(_, dt)| !matches!(dt, DataType::Primitive(primitive) if is_forbidden_typescript_bigint(primitive)),
    );
    dts
}

pub fn sanitize_typescript_bigints_in_types(types: Types) -> Types {
    types.map(|mut ty| {
        sanitize_typescript_bigints_in_datatype(ty.ty_mut());
        ty
    })
}

fn is_forbidden_typescript_bigint(primitive: &Primitive) -> bool {
    matches!(
        primitive,
        Primitive::i64
            | Primitive::i128
            | Primitive::isize
            | Primitive::u64
            | Primitive::u128
            | Primitive::usize
    )
}

fn sanitize_typescript_bigints_in_datatype(dt: &mut DataType) {
    match dt {
        DataType::Primitive(primitive) => {
            *primitive = match primitive.clone() {
                Primitive::i64 | Primitive::i128 | Primitive::isize => Primitive::i32,
                Primitive::u64 | Primitive::u128 | Primitive::usize => Primitive::u32,
                primitive => primitive,
            };
        }
        DataType::List(list) => sanitize_typescript_bigints_in_datatype(list.ty_mut()),
        DataType::Map(map) => {
            sanitize_typescript_bigints_in_datatype(map.key_ty_mut());
            sanitize_typescript_bigints_in_datatype(map.value_ty_mut());
        }
        DataType::Struct(strct) => sanitize_typescript_bigints_in_fields(strct.fields_mut()),
        DataType::Enum(enm) => {
            for (_, variant) in enm.variants_mut() {
                sanitize_typescript_bigints_in_fields(variant.fields_mut());
            }
        }
        DataType::Tuple(tuple) => tuple
            .elements_mut()
            .iter_mut()
            .for_each(sanitize_typescript_bigints_in_datatype),
        DataType::Nullable(inner) => sanitize_typescript_bigints_in_datatype(inner),
        DataType::Reference(_) => {}
    }
}

fn sanitize_typescript_bigints_in_fields(fields: &mut Fields) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => fields
            .fields_mut()
            .iter_mut()
            .filter_map(|field| field.ty_mut())
            .for_each(sanitize_typescript_bigints_in_datatype),
        Fields::Named(fields) => fields
            .fields_mut()
            .iter_mut()
            .filter_map(|(_, field)| field.ty_mut())
            .for_each(sanitize_typescript_bigints_in_datatype),
    }
}
