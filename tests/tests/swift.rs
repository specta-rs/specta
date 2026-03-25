use serde::{Deserialize, Serialize};
use specta::{ResolvedTypes, Type, Types};
use specta_swift::Swift;

fn phase_collections(
    types: Types,
) -> [(&'static str, Result<ResolvedTypes, specta_serde::Error>); 3] {
    [
        ("raw", Ok(ResolvedTypes::from_resolved_types(types.clone()))),
        ("serde", specta_serde::apply(types.clone())),
        ("serde_phases", specta_serde::apply_phases(types)),
    ]
}

fn phase_output(types: Result<ResolvedTypes, specta_serde::Error>) -> String {
    types.map_or_else(
        |err| format!("ERROR: {err}"),
        |types| {
            Swift::default()
                .export(&types)
                .unwrap_or_else(|err| format!("ERROR: {err}"))
        },
    )
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

    for (mode, result) in phase_collections(types.clone()) {
        insta::assert_snapshot!(format!("swift-export-{mode}"), phase_output(result));
    }
}
