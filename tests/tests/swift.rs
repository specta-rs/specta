use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use specta::{Format, Type, Types};
use specta_swift::Swift;

struct IdentityFormat;

impl Format for IdentityFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        _: &Types,
        dt: &specta::datatype::DataType,
    ) -> Result<Cow<'_, specta::datatype::DataType>, specta::FormatError> {
        Ok(Cow::Owned(dt.clone()))
    }
}

fn phase_collections(types: Types) -> Vec<(&'static str, Box<dyn Format>, Types)> {
    vec![
        ("raw", Box::new(IdentityFormat), types.clone()),
        ("serde", Box::new(specta_serde::Format), types.clone()),
        ("serde_phases", Box::new(specta_serde::PhasesFormat), types),
    ]
}

fn phase_output(types: &Types, format: impl Format + 'static) -> String {
    Swift::default().export(types, format).unwrap()
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "snake_case")]
enum JobStatus {
    Queued,
    PendingApproval,
}

#[derive(Type)]
#[specta(collect = false)]
enum RegularEnum {
    VariantOne,
    VariantTwo,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "snake_case")]
enum MixedEnum {
    Unit,
    WithData(String),
}

#[test]
fn swift_export() {
    let types = Types::default()
        .register::<JobStatus>()
        .register::<RegularEnum>()
        .register::<MixedEnum>();

    for (mode, format, types) in phase_collections(types) {
        insta::assert_snapshot!(format!("swift-export-{mode}"), phase_output(&types, format));
    }
}

/// A trailing `#[serde(default)]` tuple element is optional on deserialize
/// (serde accepts `[1]`) — the deserialize half must mark it optional
/// instead of requiring every element.
#[derive(Type, Deserialize, Serialize)]
#[specta(collect = false)]
struct SwiftTupleDefault(u8, #[serde(default)] u8);

#[test]
fn swift_tuple_default_phases() {
    let rendered = Swift::default()
        .export(
            &Types::default().register::<SwiftTupleDefault>(),
            specta_serde::PhasesFormat,
        )
        .expect("Swift should support defaulted tuple elements under PhasesFormat");

    insta::assert_snapshot!("swift-tuple-default-phases", rendered);
    assert!(
        rendered.contains("field1: UInt8?"),
        "the defaulted element must be optional in the deserialize half: {rendered}"
    );
}

/// Control: Swift skips `ty: None` marker slots, so a skip-reduced defaulted
/// tuple renders only the live element (optional on deserialize).
#[derive(Type, Deserialize, Serialize)]
#[specta(collect = false)]
struct SwiftSkipSlotTuple(#[serde(skip)] u8, #[serde(default)] u8);

#[test]
fn swift_skip_slot_tuple_phases() {
    let rendered = Swift::default()
        .export(
            &Types::default().register::<SwiftSkipSlotTuple>(),
            specta_serde::PhasesFormat,
        )
        .expect("Swift should support skip-reduced defaulted tuple structs");

    assert!(
        !rendered.contains("field0"),
        "the off-wire skipped slot must not render: {rendered}"
    );
    assert!(
        rendered.contains("field1: UInt8?"),
        "the live element is optional in the deserialize half: {rendered}"
    );
}
