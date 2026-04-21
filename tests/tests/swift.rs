use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use specta::{Format, Type, Types};
use specta_swift::Swift;

fn phase_collections(types: Types) -> [(&'static str, Format, Types); 3] {
    // Don't copy this format pattern anywhere else!!!
    // It's not correct but it's useful specifically here!!!
    fn identity_types(types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Borrowed(types))
    }

    fn identity_datatype<'a>(
        _: &'a Types,
        dt: &'a specta::datatype::DataType,
    ) -> Result<Cow<'a, specta::datatype::DataType>, specta::FormatError> {
        Ok(Cow::Borrowed(dt))
    }

    let identity_format = Format::new(identity_types, identity_datatype);

    [
        ("raw", identity_format, types.clone()),
        ("serde", specta_serde::format, types.clone()),
        ("serde_phases", specta_serde::format_phases, types),
    ]
}

fn phase_output(types: &Types, format: Format) -> String {
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
