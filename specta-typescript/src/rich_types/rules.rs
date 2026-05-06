use crate::rich_types::RichTypesConfiguration;

impl Default for RichTypesConfiguration {
    fn default() -> Self {
        Self {
            rules: vec![
                // TODO
            ],
            lossless_bigint: false,
            lossless_floats: false,
        }
    }
}
