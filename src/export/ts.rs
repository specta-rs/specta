use std::collections::BTreeMap;

use crate::ts::{self, ExportConfig, ExportError};

use super::get_types;

/// Exports all types in the [`TYPES`](static@crate::export::TYPES) map to the provided TypeScript file.
pub fn ts(path: &str) -> Result<(), ExportError> {
    ts_with_cfg(path, &ExportConfig::default())
}

/// Exports all types in the [`TYPES`](static@crate::export::TYPES) map to the provided TypeScript file but allow you to provide a configuration for the exporter.
pub fn ts_with_cfg(path: &str, conf: &ExportConfig) -> Result<(), ExportError> {
    let mut out = "// This file has been generated by Specta. DO NOT EDIT.\n\n".to_string();

    // We sort by name to detect duplicate types BUT also to ensure the output is deterministic. The SID can change between builds so is not suitable for this.
    let types = get_types()
        .filter(|(_, v)| match v {
            Some(_) => true,
            None => {
                unreachable!("Placeholder type should never be returned from the Specta functions!")
            }
        })
        .collect::<BTreeMap<_, _>>();

    // This is a clone of `detect_duplicate_type_names` but using a `BTreeMap` for deterministic ordering
    let mut map = BTreeMap::new();
    for (sid, dt) in &types {
        match dt {
            Some(dt) => {
                if let Some(ext) = &dt.ext {
                    if let Some((existing_sid, existing_impl_location)) =
                        map.insert(dt.name.clone(), (sid, ext.impl_location))
                    {
                        if existing_sid != sid {
                            return Err(ExportError::DuplicateTypeName(
                                dt.name.clone(),
                                ext.impl_location,
                                existing_impl_location,
                            ));
                        }
                    }
                }
            }
            None => unreachable!(),
        }
    }

    for (_, typ) in types.iter() {
        out += &ts::export_named_datatype(
            conf,
            match typ {
                Some(v) => v,
                None => unreachable!(),
            },
            &types,
        )?;
        out += "\n\n";
    }

    std::fs::write(path, out).map_err(Into::into)
}
