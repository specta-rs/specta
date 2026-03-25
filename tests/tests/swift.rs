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
fn swift_export_raw_and_serde_modes() {
    let types = Types::default()
        .register::<JobStatus>()
        .register::<RegularEnum>()
        .register::<MixedEnum>();

    for (mode, result) in phase_collections(types.clone()) {
        let output = Swift::default().export(&result.unwrap()).unwrap();

        match mode {
            "raw" => {
                assert!(
                    output.contains("enum JobStatus: Codable"),
                    "{mode}\n{output}"
                );
                assert!(output.contains("case queued"), "{mode}\n{output}");
                assert!(
                    !output.contains("enum JobStatus: String, Codable"),
                    "{mode}\n{output}"
                );

                assert!(
                    output.contains("enum RegularEnum: Codable"),
                    "{mode}\n{output}"
                );
                assert!(output.contains("case variantOne"), "{mode}\n{output}");

                assert!(output.contains("enum MixedEnum"), "{mode}\n{output}");
                assert!(output.contains("case unit"), "{mode}\n{output}");
                assert!(output.contains("case withData(String)"), "{mode}\n{output}");
            }
            "serde" | "serde_phases" => {
                assert!(
                    output.contains("enum JobStatus: String, Codable"),
                    "{mode}\n{output}"
                );
                assert!(
                    output.contains("case queued = \"queued\""),
                    "{mode}\n{output}"
                );
                assert!(
                    output.contains("case pendingApproval = \"pending_approval\""),
                    "{mode}\n{output}"
                );

                assert!(
                    output.contains("enum RegularEnum: String, Codable"),
                    "{mode}\n{output}"
                );
                assert!(
                    output.contains("case variantOne = \"VariantOne\""),
                    "{mode}\n{output}"
                );
            }
            _ => unreachable!(),
        }
    }
}
