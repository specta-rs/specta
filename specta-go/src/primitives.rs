use std::collections::HashSet;

use specta::{
    Types,
    datatype::{DataType, Enum, Fields, Generic, NamedDataType, Primitive, Reference, Struct},
};

use crate::{Error, Go, reserved_names::RESERVED_GO_NAMES};

/// Tracks necessary Go imports (e.g. "time", "encoding/json")
#[derive(Default)]
pub struct GoContext {
    pub imports: HashSet<String>,
}

impl GoContext {
    pub fn add_import(&mut self, import: &str) {
        self.imports.insert(import.to_string());
    }
}

pub fn export(
    exporter: &Go,
    types: &Types,
    ndt: &NamedDataType,
    ctx: &mut GoContext,
) -> Result<String, Error> {
    let mut s = String::new();

    let docs = &ndt.docs;
    if !docs.is_empty() {
        for line in docs.lines() {
            s.push_str("// ");
            s.push_str(line);
            s.push('\n');
        }
    }

    let name = to_pascal_case(&ndt.name);
    if RESERVED_GO_NAMES.contains(&name.as_str()) {
        return Err(Error::ForbiddenName {
            path: ndt.name.to_string(),
            name: ndt.name.to_string(),
        });
    }

    s.push_str("type ");
    s.push_str(&name);

    if !ndt.generics.is_empty() {
        s.push('[');
        for (i, g) in ndt.generics.iter().enumerate() {
            if i != 0 {
                s.push_str(", ");
            }
            s.push_str(g.name.as_ref());
            s.push_str(" any");
        }
        s.push(']');
    }
    s.push(' ');

    let generic_names = ndt
        .generics
        .iter()
        .map(|generic| generic.reference())
        .collect::<Vec<_>>();

    let Some(ty) = &ndt.ty else {
        return Ok(String::new());
    };

    match ty {
        DataType::Struct(st) => {
            s.push_str("struct {\n");
            struct_fields(
                &mut s,
                exporter,
                types,
                &generic_names,
                st,
                vec![ndt.name.to_string()],
                ctx,
            )?;
            s.push('}');
        }
        DataType::Enum(e) => {
            s.push_str("struct {\n");
            enum_variants(
                &mut s,
                exporter,
                types,
                &generic_names,
                e,
                vec![ndt.name.to_string()],
                ctx,
            )?;
            s.push('}');
        }
        DataType::Tuple(t) => {
            if t.elements.len() == 1 {
                datatype(
                    &mut s,
                    exporter,
                    types,
                    &generic_names,
                    &t.elements[0],
                    vec![ndt.name.to_string(), "0".into()],
                    ctx,
                )?;
            } else {
                s.push_str("[]any");
            }
        }
        _ => {
            datatype(
                &mut s,
                exporter,
                types,
                &generic_names,
                ty,
                vec![ndt.name.to_string()],
                ctx,
            )?;
        }
    }
    s.push('\n');

    Ok(s)
}

fn struct_fields(
    s: &mut String,
    exporter: &Go,
    types: &Types,
    generic_names: &[Generic],
    st: &Struct,
    location: Vec<String>,
    ctx: &mut GoContext,
) -> Result<(), Error> {
    match &st.fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => {
            for (i, field) in fields.fields.iter().enumerate() {
                s.push('\t');
                s.push_str(&format!("Field{}", i));
                s.push(' ');

                if field.optional {
                    s.push('*');
                }

                if let Some(ty) = field.ty.as_ref() {
                    let mut location = location.clone();
                    location.push(i.to_string());
                    datatype(s, exporter, types, generic_names, ty, location, ctx)?;
                } else {
                    s.push_str("any");
                }
                s.push('\n');
            }
        }
        Fields::Named(fields) => {
            for (name, field) in &fields.fields {
                let docs = &field.docs;
                if !docs.is_empty() {
                    s.push_str("\t// ");
                    s.push_str(docs.replace('\n', "\n\t// ").trim());
                    s.push('\n');
                }

                s.push('\t');
                s.push_str(&to_pascal_case(name));
                s.push(' ');

                if field.optional {
                    s.push('*');
                }

                if let Some(ty) = field.ty.as_ref() {
                    let mut location = location.clone();
                    location.push(name.to_string());
                    datatype(s, exporter, types, generic_names, ty, location, ctx)?;
                } else {
                    s.push_str("any");
                }

                if field.optional {
                    s.push_str(&format!(" `json:\"{},omitempty\"`\n", name));
                } else {
                    s.push_str(&format!(" `json:\"{}\"`\n", name));
                }
            }
        }
    }
    Ok(())
}

fn enum_variants(
    s: &mut String,
    exporter: &Go,
    types: &Types,
    generic_names: &[Generic],
    e: &Enum,
    location: Vec<String>,
    ctx: &mut GoContext,
) -> Result<(), Error> {
    for (name, variant) in &e.variants {
        let docs = &variant.docs;
        if !docs.is_empty() {
            s.push_str("\t// ");
            s.push_str(docs);
            s.push('\n');
        }

        s.push('\t');
        s.push_str(&to_pascal_case(name));
        s.push(' ');
        s.push('*');

        match &variant.fields {
            Fields::Unit => s.push_str("bool"),
            Fields::Unnamed(f) => {
                s.push_str("struct {\n");
                for (i, field) in f.fields.iter().enumerate() {
                    s.push_str("\t\tValue");
                    s.push_str(&i.to_string());
                    s.push(' ');
                    if let Some(ty) = field.ty.as_ref() {
                        let mut location = location.clone();
                        location.push(name.to_string());
                        location.push(i.to_string());
                        datatype(s, exporter, types, generic_names, ty, location, ctx)?;
                    } else {
                        s.push_str("any");
                    }
                    s.push_str(&format!(" `json:\"{}\"`\n", i));
                }
                s.push('\t');
                s.push('}');
            }
            Fields::Named(f) => {
                s.push_str("struct {\n\t");
                let mut fill_in = Struct::unit();
                fill_in.fields = Fields::Named(f.clone());

                let mut location = location.clone();
                location.push(name.to_string());
                struct_fields(s, exporter, types, generic_names, &fill_in, location, ctx)?;
                s.push('\t');
                s.push('}');
            }
        }
        s.push_str(&format!(" `json:\"{},omitempty\"`\n", name));
    }
    Ok(())
}

fn datatype(
    s: &mut String,
    exporter: &Go,
    types: &Types,
    generic_names: &[Generic],
    dt: &DataType,
    location: Vec<String>,
    ctx: &mut GoContext,
) -> Result<(), Error> {
    match dt {
        DataType::Primitive(p) => match p {
            Primitive::i8 => s.push_str("int8"),
            Primitive::i16 => s.push_str("int16"),
            Primitive::i32 => s.push_str("int32"),
            Primitive::i64 | Primitive::isize => s.push_str("int64"),
            Primitive::u8 => s.push_str("uint8"),
            Primitive::u16 => s.push_str("uint16"),
            Primitive::u32 => s.push_str("uint32"),
            Primitive::u64 | Primitive::usize => s.push_str("uint64"),
            Primitive::f16 | Primitive::f32 => s.push_str("float32"),
            Primitive::f64 | Primitive::f128 => s.push_str("float64"),
            Primitive::bool => s.push_str("bool"),
            Primitive::str | Primitive::char => s.push_str("string"),
            Primitive::i128 | Primitive::u128 => {
                return Err(Error::BigIntForbidden {
                    path: location.join("."),
                });
            }
        },
        DataType::Nullable(t) => {
            s.push('*');
            datatype(s, exporter, types, generic_names, t, location, ctx)?;
        }
        DataType::Map(m) => {
            s.push_str("map[");
            datatype(
                s,
                exporter,
                types,
                generic_names,
                m.key_ty(),
                location.clone(),
                ctx,
            )?;
            s.push(']');
            datatype(
                s,
                exporter,
                types,
                generic_names,
                m.value_ty(),
                location,
                ctx,
            )?;
        }
        DataType::List(l) => {
            s.push_str("[]");
            datatype(s, exporter, types, generic_names, &l.ty, location, ctx)?;
        }
        DataType::Tuple(t) => {
            if t.elements.is_empty() {
                s.push_str("struct{}");
            } else {
                s.push_str("[]any");
            }
        }
        DataType::Struct(st) => {
            s.push_str("struct {\n");
            struct_fields(s, exporter, types, generic_names, st, location, ctx)?;
            s.push('}');
        }
        DataType::Enum(e) => {
            s.push_str("struct {\n");
            enum_variants(s, exporter, types, generic_names, e, location, ctx)?;
            s.push('}');
        }
        DataType::Reference(r) => match r {
            Reference::Named(r) => {
                let ndt = r.get(types).ok_or_else(|| Error::ForbiddenName {
                    path: "lookup".into(),
                    name: "missing_reference_in_collection".into(),
                })?;

                s.push_str(&to_pascal_case(&ndt.name));

                let generics = r.generics();
                if !generics.is_empty() {
                    s.push('[');
                    for (i, (_, g)) in generics.iter().enumerate() {
                        if i != 0 {
                            s.push_str(", ");
                        }
                        let mut location = location.clone();
                        location.push(format!("generic{}", i));
                        datatype(s, exporter, types, generic_names, g, location, ctx)?;
                    }
                    s.push(']');
                }
            }
            Reference::Opaque(o) => match o.type_name() {
                "String" | "char" => s.push_str("string"),
                "bool" => s.push_str("bool"),
                "i8" | "i16" | "i32" | "isize" => s.push_str("int"),
                "u8" | "u16" | "u32" | "usize" => s.push_str("uint"),
                "i64" => s.push_str("int64"),
                "u64" => s.push_str("uint64"),
                "f32" => s.push_str("float32"),
                "f64" => s.push_str("float64"),
                "SystemTime" | "DateTime" => {
                    ctx.add_import("time");
                    s.push_str("time.Time");
                }
                "Duration" => {
                    ctx.add_import("time");
                    s.push_str("time.Duration");
                }
                _ => s.push_str("any"),
            },
        },
        DataType::Generic(g) => {
                let name = generic_names
                    .iter()
                    .find(|candidate| candidate.reference() == *g)
                    .map(|generic| generic.name().as_ref())
                    .unwrap_or("any");
                s.push_str(name);
        }
        DataType::Intersection(_) => s.push_str("any"),
    }
    Ok(())
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut next_upper = true;
    for c in s.chars() {
        if c == '_' {
            next_upper = true;
        } else if next_upper {
            result.push(c.to_ascii_uppercase());
            next_upper = false;
        } else {
            result.push(c);
        }
    }
    result
}
