use crate::{
    define,
    rich_types::{DataTypeFn, RichTypesConfiguration, Rule, Transform},
};

impl Default for RichTypesConfiguration {
    fn default() -> Self {
        Self {
            rules: vec![
                // Uint8Array
                Rule {
                    name: "Bytes".into(),
                    module_path: "bytes".into(),
                    data_type: DataTypeFn::new(|_| define("Uint8Array").into()),
                    serialize: Some(Transform::new(|i| format!("[...{i}]"))),
                    deserialize: Some(Transform::new(|i| format!("new Uint8Array({i})"))),
                },
                Rule {
                    name: "BytesMut".into(),
                    module_path: "bytes".into(),
                    data_type: DataTypeFn::new(|_| define("Uint8Array").into()),
                    serialize: Some(Transform::new(|i| format!("[...{i}]"))),
                    deserialize: Some(Transform::new(|i| format!("new Uint8Array({i})"))),
                },
                // Date
                Rule {
                    name: "DateTime".into(),
                    module_path: "chrono".into(),
                    data_type: DataTypeFn::new(|_| define("Date").into()),
                    serialize: None,
                    deserialize: Some(Transform::new(|i| format!("new Date({i})"))),
                },
                // TODO: `NaiveDate`, `NaiveDateTime`, `DateTime<FixedOffset>`
                // https://chatgpt.com/c/69fae901-2590-839c-8174-cfca70cc23bc
            ],
            remapper: Default::default(),
            lossless_bigint: false,
            lossless_floats: false,
        }
    }
}
