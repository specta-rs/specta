use serde::Serialize;
use specta::{Format as _, Type, Types, datatype::{DataType, Reference}};

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum LegitimatePhaseNames {
    Serialize(String),
    Deserialize(bool),
}

#[test]
fn user_enums_that_look_like_phase_wrappers_are_preserved() {
    let mut types = Types::default();
    let dt = LegitimatePhaseNames::definition(&mut types);
    let resolved = specta_serde::PhasesFormat
        .map_types(&types)
        .expect("the enum should format")
        .into_owned();

    let selected = specta_serde::select_phase_datatype(
        &dt,
        &resolved,
        specta_serde::Phase::Serialize,
    );
    let DataType::Reference(Reference::Named(reference)) = selected else {
        panic!("the user's enum must not be treated as a generated phase wrapper");
    };
    assert_eq!(
        resolved.get(&reference).map(|ndt| ndt.name.as_ref()),
        Some("LegitimatePhaseNames")
    );
}
