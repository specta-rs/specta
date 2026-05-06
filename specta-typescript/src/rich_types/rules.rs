use std::sync::Arc;

use crate::rich_types::{RichTypesConfiguration, Rule};

impl Default for RichTypesConfiguration {
    fn default() -> Self {
        Self {
            rules: vec![
                Rule {
                    name: "Bytes".into(),
                    module_path: "bytes".into(),
                    typ: Arc::new(|dt| dt),
                    runtime: Arc::new(|i| format!("new Uint8Array({i})")),
                },
                Rule {
                    name: "BytesMut".into(),
                    module_path: "bytes".into(),
                    typ: Arc::new(|dt| dt),
                    runtime: Arc::new(|i| format!("new Uint8Array({i})")),
                },
            ],
            remapper: None,
            lossless_bigint: false,
            lossless_floats: false,
        }
    }
}
