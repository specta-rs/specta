use serde::Deserialize;
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type, Deserialize)]
#[specta(collect = false)]
#[serde(variant_identifier, rename_all = "snake_case")]
enum VariantIdentifier {
    HttpStatus,
    #[serde(alias = "legacy")]
    LegacyName,
}

#[derive(Type, Deserialize)]
#[specta(collect = false)]
#[serde(field_identifier, rename_all = "snake_case")]
enum FieldIdentifier {
    FirstName,
    LastName,
    Other(bool),
}

#[test]
fn identifier_apply_requires_phases() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<VariantIdentifier>(),
            specta_serde::Format,
        )
        .expect_err("variant_identifier should require PhasesFormat");
    assert!(
        err.to_string()
            .contains("identifier enums require `PhasesFormat`")
    );

    let err = Typescript::default()
        .export(
            &Types::default().register::<FieldIdentifier>(),
            specta_serde::Format,
        )
        .expect_err("field_identifier should require PhasesFormat");
    assert!(
        err.to_string()
            .contains("identifier enums require `PhasesFormat`")
    );
}

#[test]
fn identifier_phases_format_exports_deserialize_union() {
    let variant_ts = Typescript::default()
        .export(
            &Types::default().register::<VariantIdentifier>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-identifiers-variant-typescript", variant_ts);

    let field_ts = Typescript::default()
        .export(
            &Types::default().register::<FieldIdentifier>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-identifiers-field-typescript", field_ts);
}
