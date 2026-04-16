use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_swift::Swift;

fn phase_collections(
    types: Types,
) -> [(&'static str, Result<Types, specta_serde::FormatError>); 3] {
    [
        ("raw", Ok(types.clone())),
        ("serde", crate::serde(types.clone())),
        ("serde_phases", crate::serde_phases(types)),
    ]
}

fn phase_output(types: Result<Types, specta_serde::FormatError>) -> String {
    types.map_or_else(
        |err| format!("ERROR: {err}"),
        |types| {
            Swift::default()
                .export(&types, specta_serde::format)
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
