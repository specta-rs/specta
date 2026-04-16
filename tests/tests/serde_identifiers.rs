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
    let err = crate::serde(Types::default().register::<VariantIdentifier>())
        .expect_err("variant_identifier should require format_phases");
    assert!(
        err.to_string()
            .contains("identifier enums require `format_phases`")
    );

    let err = crate::serde(Types::default().register::<FieldIdentifier>())
        .expect_err("field_identifier should require format_phases");
    assert!(
        err.to_string()
            .contains("identifier enums require `format_phases`")
    );
}

#[test]
fn identifier_format_phases_exports_deserialize_union() {
    let variant_types = crate::serde_phases(Types::default().register::<VariantIdentifier>())
        .expect("variant_identifier should be supported by format_phases");
    let variant_ts = Typescript::default()
        .export(&variant_types, crate::raw_format)
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-identifiers-variant-typescript", variant_ts);

    let field_types = crate::serde_phases(Types::default().register::<FieldIdentifier>())
        .expect("field_identifier should be supported by format_phases");
    let field_ts = Typescript::default()
        .export(&field_types, crate::raw_format)
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-identifiers-field-typescript", field_ts);
}
