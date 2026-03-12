use std::iter;

use specta::{
    ResolvedTypes, Type, Types,
    datatype::{DataType, Reference},
};
use specta_zod::{BigIntExportBehavior, Layout, Zod, primitives};

#[test]
fn zod_export_smoke() {
    #[derive(Type)]
    struct Inner {
        value: String,
    }

    #[derive(Type)]
    struct Demo {
        inner: Inner,
        count: i32,
        maybe: Option<String>,
    }

    let types = Types::default().register::<Demo>();
    let resolved = ResolvedTypes::from_resolved_types(types);

    let out = Zod::default()
        .bigint(BigIntExportBehavior::Number)
        .export(&resolved)
        .unwrap();

    assert!(out.contains("import { z } from \"zod\";"));
    assert!(out.contains("export const DemoSchema"));
    assert!(out.contains("export type Demo = z.infer<typeof DemoSchema>;"));
}

#[test]
fn zod_primitives_smoke() {
    let (types, dts) = crate::types();
    let resolved = ResolvedTypes::from_resolved_types(types);

    let zod = Zod::default().bigint(BigIntExportBehavior::Number);

    for (_, ty) in &dts {
        let rendered = primitives::inline(&zod, &resolved, ty).unwrap();
        assert!(!rendered.is_empty());
    }

    let ndt = dts
        .iter()
        .find_map(|(_, ty)| match ty {
            DataType::Reference(Reference::Named(r)) => r.get(resolved.as_types()),
            _ => None,
        })
        .unwrap();

    let rendered = primitives::export(&zod, &resolved, iter::once(ndt), "").unwrap();
    assert!(rendered.contains("Schema"));
}

#[test]
fn zod_layout_files_errors_on_export() {
    let types = Types::default();
    let resolved = ResolvedTypes::from_resolved_types(types);

    let err = Zod::default()
        .layout(Layout::Files)
        .export(&resolved)
        .unwrap_err();
    assert!(err.to_string().contains("Unable to export layout Files"));
}
