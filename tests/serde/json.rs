use specta::ts::{BigIntExportBehavior, ExportConfig};

use crate::ts::{assert_ts, assert_ts_export};

#[test]
fn serde_json() {
    assert_eq!(
        specta::ts::inline::<serde_json::Number>(
            &ExportConfig::default().bigint(BigIntExportBehavior::Number)
        ),
        Ok("number".into())
    );
    assert_ts!(serde_json::Map<String, String>, "{ [key in string]: string }");

    assert_eq!(
        specta::ts::inline::<serde_json::Value>(
            &ExportConfig::default().bigint(BigIntExportBehavior::Number)
        ),
        Ok(
            // TODO: Can we have `#[specta(inline = false)]` for this so it's `JsonValue`???
            "null | boolean | number | string | JsonValue[] | { [key in string]: JsonValue }"
                .into()
        )
    );

    // assert_ts!(serde_json::Value, ""); // TODO: This literally can't work
    // assert_ts_export!(serde_json::Value, "");
}
