use specta_serde::SerdeMode;
use specta_typescript::{BigIntExportBehavior, JSDoc};

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

// TODO: BigInt checks
// TODO: Confirm different layouts
// TODO: Unit test JSDoc and other languages

// TODO: Ensure this is feature matching with the Typescript testing
