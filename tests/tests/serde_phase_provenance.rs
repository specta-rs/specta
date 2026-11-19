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

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct GenericPhaseItem<T> {
    value: Option<T>,
}

#[test]
fn phase_selection_keeps_concrete_generic_arguments() {
    let mut types = Types::default();
    let dt = GenericPhaseItem::<String>::definition(&mut types);
    let resolved = specta_serde::PhasesFormat
        .map_types(&types)
        .expect("the generic type should format")
        .into_owned();

    let selected = specta_serde::select_phase_datatype(
        &dt,
        &resolved,
        specta_serde::Phase::Serialize,
    );
    let DataType::Reference(Reference::Named(reference)) = selected else {
        panic!("expected a named phase projection");
    };
    let specta::datatype::NamedReferenceType::Reference { generics, .. } = reference.inner else {
        panic!("expected a reference projection");
    };
    let argument = generics
        .first()
        .map(|(_, dt)| dt)
        .expect("the projection should keep its generic argument");
    assert!(
        !matches!(argument, DataType::Generic(_)),
        "selection leaked the declaration's placeholder: {argument:?}"
    );
    assert_eq!(
        argument,
        &<String as Type>::definition(&mut Types::default())
    );
}
