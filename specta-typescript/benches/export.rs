//! Dependency-free end-to-end exporter benchmark. See ../BENCHMARKS.md.
use std::{
    borrow::Cow,
    hint::black_box,
    time::{Duration, Instant},
};

use specta::{
    Format, Type, Types,
    datatype::{DataType, Enum, Field, NamedDataType, Primitive, Struct, Variant},
};
use specta_typescript::{JSDoc, Layout, Typescript};

fn named(types: &mut Types, name: String, ty: DataType) -> NamedDataType {
    NamedDataType::new(name, types, |_, ndt| {
        ndt.module_path = "bench::models".into();
        ndt.ty = Some(ty);
    })
}

fn records(count: usize) -> Types {
    let mut types = Types::default();
    let mut previous = Primitive::str.into();
    for i in 0..count {
        let ty = Struct::named()
            .field("id", Field::new(Primitive::u32.into()))
            .field("name", Field::new(Primitive::str.into()))
            .field("enabled", Field::new(Primitive::bool.into()))
            .field("parent", Field::new(DataType::Nullable(Box::new(previous))))
            .build();
        previous = named(&mut types, format!("Record{i:04}"), ty)
            .reference(vec![])
            .into();
    }
    types
}

fn unions(count: usize, linked: bool) -> Types {
    let mut types = Types::default();
    let template = Types::default().register::<UnionTemplate>();
    let attributes = template
        .into_unsorted_iter()
        .find_map(|ndt| match &ndt.ty {
            Some(DataType::Enum(enm)) => Some(enm.attributes.clone()),
            _ => None,
        })
        .expect("the template contains an enum");
    let mut previous = Primitive::str.into();
    for i in (0..count).rev() {
        let mut enm = Enum::default();
        if linked {
            enm.attributes = attributes.clone();
        }
        enm.variants.push((
            "Value".into(),
            Variant::unnamed().field(Field::new(previous)).build(),
        ));
        enm.variants.push(("Empty".into(), Variant::unit()));
        let ndt = named(&mut types, format!("Union{i:04}"), enm.into());
        previous = if linked {
            ndt.reference(vec![]).into()
        } else {
            Primitive::str.into()
        };
    }
    types
}

fn cycles(count: usize) -> Types {
    let mut types = unions(count, true);
    let first = types
        .into_unsorted_iter()
        .find(|ndt| ndt.name == "Union0000")
        .expect("the ring contains its first node")
        .reference(vec![]);
    let last = format!("Union{:04}", count - 1);
    types.iter_mut(|ndt| {
        if ndt.name == last
            && let Some(DataType::Enum(enm)) = &mut ndt.ty
        {
            enm.variants[0].1 = Variant::unnamed()
                .field(Field::new(first.clone().into()))
                .build();
        }
    });
    types
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum UnionTemplate {
    Value(String),
    Empty,
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
struct Page<T> {
    items: Vec<T>,
    next: Option<String>,
    total: u32,
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
struct User {
    id: String,
    name: String,
    roles: Vec<String>,
    address: Option<Address>,
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
struct Address {
    street: String,
    city: String,
    country: String,
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
enum Event {
    Created(User),
    Deleted { id: String },
    Renamed { id: String, name: String },
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum Recursive<T> {
    Recur(Box<Recursive<T>>),
    Leaf(T),
}

fn generic_api(count: usize) -> Types {
    let mut types = Types::default();
    let recursive = Recursive::<String>::definition(&mut types);
    for i in 0..count {
        named(
            &mut types,
            format!("Response{i:04}"),
            Struct::named()
                .field("payload", Field::new(recursive.clone()))
                .field("request_id", Field::new(Primitive::str.into()))
                .build(),
        );
    }
    types
}

fn api(count: usize) -> Types {
    let mut types = Types::default()
        .register::<Page<User>>()
        .register::<Page<Event>>();
    let page = <Page<User>>::definition(&mut types);
    let event = Event::definition(&mut types);
    for i in 0..count {
        let ty = Struct::named()
            .field("users", Field::new(page.clone()))
            .field("event", Field::new(event.clone()))
            .field(
                "metadata",
                Field::new(<std::collections::HashMap<String, String>>::definition(
                    &mut types,
                )),
            )
            .build();
        named(&mut types, format!("Response{i:04}"), ty);
    }
    types
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Pending,
    Active,
    Archived,
}

fn maps(count: usize) -> Types {
    let mut types = Types::default();
    let by_status = <std::collections::HashMap<Status, User>>::definition(&mut types);
    let by_id = <std::collections::HashMap<u32, User>>::definition(&mut types);
    let metadata = <std::collections::HashMap<String, String>>::definition(&mut types);
    for i in 0..count {
        named(
            &mut types,
            format!("Index{i:04}"),
            Struct::named()
                .field("by_status", Field::new(by_status.clone()))
                .field("by_id", Field::new(by_id.clone()))
                .field("metadata", Field::new(metadata.clone()))
                .build(),
        );
    }
    types
}

fn checksum(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}

#[derive(Default)]
struct Prepared(Types);

impl Format for Prepared {
    fn map_types(&self, _: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Borrowed(&self.0))
    }

    fn map_type(&self, _: &Types, dt: &DataType) -> Result<Cow<'_, DataType>, specta::FormatError> {
        Ok(Cow::Owned(dt.clone()))
    }
}

fn measure(name: &str, mut export: impl FnMut() -> String, milliseconds: u64) {
    let expected = export();
    // Hashing and equality checks are outside timed regions.
    let hash = checksum(&expected);
    let mut times = Vec::new();
    for _ in 0..7 {
        let start = Instant::now();
        let mut iterations = 0;
        while start.elapsed() < Duration::from_millis(milliseconds) {
            black_box(export());
            iterations += 1;
        }
        times.push(start.elapsed().as_secs_f64() * 1e6 / f64::from(iterations));
    }
    assert_eq!(expected, export());
    times.sort_by(f64::total_cmp);
    println!(
        "{name:36} median_us={:12.3} min_us={:12.3} max_us={:12.3} bytes={} fnv1a={hash:016x}",
        times[3],
        times[0],
        times[6],
        expected.len()
    );
}

fn main() {
    let filter = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .unwrap_or_default();
    let milliseconds = std::env::var("SPECTA_BENCH_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(150);
    for (shape, sizes, build) in [
        (
            "records",
            &[32, 256, 1024][..],
            records as fn(usize) -> Types,
        ),
        (
            "enums",
            &[32, 256, 1024][..],
            (|n| unions(n, false)) as fn(usize) -> Types,
        ),
        (
            "chain",
            &[32, 128, 512, 1024][..],
            (|n| unions(n, true)) as fn(usize) -> Types,
        ),
        ("api", &[8, 128, 512][..], api as fn(usize) -> Types),
        ("maps", &[32, 256, 1024][..], maps as fn(usize) -> Types),
        ("cycle", &[1, 8, 32, 128][..], cycles as fn(usize) -> Types),
        (
            "generic-api",
            &[8, 128, 512][..],
            generic_api as fn(usize) -> Types,
        ),
    ] {
        for &size in sizes {
            let types = build(size);
            for (layout_name, layout) in [
                ("flat", Layout::FlatFile),
                ("namespaces", Layout::Namespaces),
            ] {
                let name = format!("{shape}/{size}/{layout_name}");
                if name.contains(&filter) {
                    measure(
                        &name,
                        || {
                            Typescript::default()
                                .layout(layout)
                                .export(&types, specta_serde::Format)
                                .expect("the TypeScript benchmark fixture exports")
                        },
                        milliseconds,
                    );
                }
            }
            let name = format!("{shape}/{size}/jsdoc");
            if name.contains(&filter) {
                measure(
                    &name,
                    || {
                        JSDoc::default()
                            .export(&types, specta_serde::Format)
                            .expect("the JSDoc benchmark fixture exports")
                    },
                    milliseconds,
                );
            }
            let name = format!("{shape}/{size}/prepared-flat");
            if name.contains(&filter) {
                let types = specta_serde::Format
                    .map_types(&types)
                    .expect("the benchmark fixture is valid for Serde")
                    .into_owned();
                let format = Prepared::default();
                measure(
                    &name,
                    || {
                        Typescript::default()
                            .export(&types, &format)
                            .expect("the prepared benchmark fixture exports")
                    },
                    milliseconds,
                );
            }
        }
    }
}
