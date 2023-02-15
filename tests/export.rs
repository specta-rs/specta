use specta::Type;

#[derive(Type)]
pub struct TypeOne {
    pub field1: String,
    pub field2: TypeTwo,
}

#[derive(Type)]
pub struct TypeTwo {
    pub my_field: String,
}

#[cfg(feature = "export")]
#[test]
#[ignore] // TODO: Fix and enable this test once `#[specta(export = false)]` is working
fn test_export_feature() {
    use specta::{
        export,
        ts::{BigIntExportBehavior, ExportConfiguration},
    };
    use std::fs;

    export::ts("./bindings.ts").unwrap();
    assert_eq!(fs::read_to_string("./bindings.ts").unwrap(), "");
    fs::remove_file("./bindings.ts").unwrap();

    export::ts_with_cfg(
        // Be aware this won't be typesafe unless your using a ser/deserializer that converts BigInt types to a number.
        &ExportConfiguration::default().bigint(BigIntExportBehavior::Number),
        "./bindings2.ts",
    )
    .unwrap();
    assert_eq!(fs::read_to_string("./bindings2.ts").unwrap(), "");
    fs::remove_file("./bindings2.ts").unwrap();
}
