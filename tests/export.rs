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

#[derive(Type)]
#[specta(export = false)]
pub struct NotExported {
    pub b: i32,
}

// TODO: This type is not gonna export correctly. Fix this. It should probally inline `NotExported`????
#[derive(Type)]
pub struct ReferingToUnexportedType {
    pub a: NotExported,
}

#[cfg(feature = "export")]
#[test]
fn test_export_feature() {
    use specta::{
        export,
        ts::{BigIntExportBehavior, ExportConfiguration},
    };
    use std::fs;

    // This test fails with `--all-features` because the uhlc library uses BigInt's and will always be registered when the `uhlc` feature is enabled.
    // export::ts("./bindings.ts").unwrap();
    // assert_eq!(fs::read_to_string("./bindings.ts").unwrap(), "");
    // fs::remove_file("./bindings.ts").unwrap();

    export::ts_with_cfg(
        "./bindings2.ts",
        // Be aware this won't be typesafe unless your using a ser/deserializer that converts BigInt types to a number.
        &ExportConfiguration::default().bigint(BigIntExportBehavior::Number),
    )
    .unwrap();
    assert_eq!(fs::read_to_string("./bindings2.ts").unwrap(), "// This file has been generated by Specta. DO NOT EDIT.\n\nexport type DAffine2 = { matrix2: DMat2; translation: DVec2 }\n\nexport type DMat2 = { x_axis: DVec2; y_axis: DVec2 }\n\nexport type DVec2 = { x: number; y: number }\n\nexport type IVec2 = { x: number; y: number }\n\nexport type ReferingToUnexportedType = { a: NotExported }\n\nexport type Timestamp = { time: number; id: number }\n\nexport type TypeOne = { field1: string; field2: TypeTwo }\n\nexport type TypeTwo = { my_field: string }\n\n");
    fs::remove_file("./bindings2.ts").unwrap();
}
