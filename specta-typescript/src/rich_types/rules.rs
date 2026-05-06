use crate::rich_types::RichTypesConfiguration;

impl Default for RichTypesConfiguration {
    fn default() -> Self {
        Self {
            rules: vec![
                // TODO
            ],
            remapper: None,
            lossless_bigint: false,
            lossless_floats: false,
        }
    }
}
