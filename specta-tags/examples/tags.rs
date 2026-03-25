//! Basic capabilities of `specta-tags`.

use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime},
};

use specta::{
    Type, Types,
    datatype::{DataType, List, Primitive},
};
use specta_tags::{Analyzer, RuntimeRequirements, RuntimeTarget, render_runtime};

#[allow(dead_code)]
#[derive(Type)]
struct FileChunk {
    created_at: SystemTime,
    payload: Vec<u8>,
    checksums: Vec<u64>,
}

#[allow(dead_code)]
#[derive(Type)]
struct TransferEvent {
    event_id: u128,
    happened_at: SystemTime,
    expires_in: Option<Duration>,
    chunks: Vec<FileChunk>,
    retries_by_region: BTreeMap<String, i64>,
}

fn main() -> Result<(), serde_json::Error> {
    let analyzer = Analyzer::with_builtins().with_list_u8_is_bytes(true);

    let bigint_spec = analyzer.analyze(
        &DataType::Primitive(Primitive::u128),
        &Types::default(),
        &[],
    );
    println!(
        "bigint ->\n{}\n",
        serde_json::to_string_pretty(&bigint_spec)?
    );

    let uint8_array_spec = analyzer.analyze(
        &DataType::List(List::new(DataType::Primitive(Primitive::u8))),
        &Types::default(),
        &[],
    );
    println!(
        "UInt8Array (Vec<u8>) ->\n{}\n",
        serde_json::to_string_pretty(&uint8_array_spec)?
    );

    let mut date_types = Types::default();
    let date_datatype = SystemTime::definition(&mut date_types);
    let date_spec = analyzer.analyze(&date_datatype, &date_types, &[]);
    println!(
        "date (SystemTime) ->\n{}\n",
        serde_json::to_string_pretty(&date_spec)?
    );

    let mut types = Types::default();
    let datatype = TransferEvent::definition(&mut types);

    let spec = analyzer.analyze(&datatype, &types, &[]);

    println!("nested object ->");
    println!("{}", serde_json::to_string_pretty(&spec)?);

    let requirements = RuntimeRequirements::from_specs([&spec]);

    println!("\n--- TypeScript Runtime ---\n");
    println!(
        "{}",
        render_runtime(RuntimeTarget::TypeScript, &requirements)
    );

    println!("\n--- JavaScript Runtime ---\n");
    println!(
        "{}",
        render_runtime(RuntimeTarget::JavaScript, &requirements)
    );

    Ok(())
}
