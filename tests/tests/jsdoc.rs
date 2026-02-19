use specta::datatype::{DataType, Reference};
use specta_serde::SerdeMode;
use specta_typescript::{BigIntExportBehavior, JSDoc, primitives};

#[test]
fn export() {
    for mode in [
        SerdeMode::Both,
        SerdeMode::Serialize,
        SerdeMode::Deserialize,
    ] {
        insta::assert_snapshot!(
            format!("inline-{}", mode.to_string().to_lowercase()),
            JSDoc::default()
                .with_serde(mode)
                .bigint(BigIntExportBehavior::Number)
                .export(&crate::types().0)
                .unwrap()
        );
    }
}

#[test]
fn primitives_export_many() {
    for mode in [
        SerdeMode::Both,
        SerdeMode::Serialize,
        SerdeMode::Deserialize,
    ] {
        let jsdoc = JSDoc::default()
            .with_serde(mode)
            .bigint(BigIntExportBehavior::Number);
        let (types, dts) = crate::types();
        let ndts = dts
            .iter()
            .filter_map(|(_, ty)| match ty {
                DataType::Reference(Reference::Named(r)) => r.get(&types),
                _ => None,
            })
            .collect::<Vec<_>>();

        insta::assert_snapshot!(
            format!("primitives-many-inline-{}", mode.to_string().to_lowercase()),
            primitives::export_many(&jsdoc, &types, ndts.into_iter()).unwrap()
        );
    }
}

// TODO: BigInt checks
// TODO: Confirm different layouts
// TODO: Unit test JSDoc and other languages

// TODO: Ensure this is feature matching with the Typescript testing
