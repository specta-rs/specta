// Regression coverage for https://github.com/specta-rs/specta/pull/640.
use specta::{
    Format, Type, Types,
    datatype::{DataType, Field, NamedDataType, Primitive, Struct},
};

fn graph(count: usize, cyclic: bool) -> Types {
    let mut types = Types::default();
    let nodes = (0..count)
        .map(|i| {
            NamedDataType::new(format!("Node{i:04}"), &mut types, |_, ndt| {
                ndt.ty = Some(Primitive::str.into());
            })
        })
        .collect::<Vec<_>>();
    types.for_each_named_mut(|ndt| {
        let i = ndt
            .name
            .trim_start_matches("Node")
            .parse::<usize>()
            .expect("node names have numeric suffixes");
        let next: DataType = nodes
            .get(i + 1)
            .or_else(|| cyclic.then_some(&nodes[0]))
            .map(|ndt| ndt.reference(vec![]).into())
            .unwrap_or(Primitive::str.into());
        let shared = if i + 1 == count {
            Primitive::bool.into()
        } else {
            nodes[count - 1].reference(vec![]).into()
        };
        ndt.ty = Some(
            Struct::named()
                .field("next", Field::new(next.clone()))
                .field("shared", Field::new(shared))
                .field("duplicate", Field::new(next))
                .build(),
        );
    });
    types
}

#[test]
fn shared_tails_and_cycles_preserve_bodies_in_both_modes() {
    // The existing recursive validator needs more than the default test-thread
    // stack for this depth, even before visit reuse.
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            for cyclic in [false, true] {
                let types = graph(128, cyclic);
                for format in [
                    &specta_serde::Format as &dyn Format,
                    &specta_serde::PhasesFormat,
                ] {
                    let mapped = format.map_types(&types).expect("the shared graph is valid");
                    let original = types.into_sorted_iter().collect::<Vec<_>>();
                    let rewritten = mapped.into_sorted_iter().collect::<Vec<_>>();
                    assert_eq!(original.len(), rewritten.len());
                    for (original, rewritten) in original.iter().zip(rewritten) {
                        assert_eq!(original.name, rewritten.name);
                        assert_eq!(original.ty, rewritten.ty);
                    }
                }
            }
        })
        .expect("the graph test thread starts")
        .join()
        .expect("the graph validation succeeds");
}

fn add_invalid_codec(types: &mut Types) {
    types.for_each_named_mut(|ndt| {
        let reference = ndt.reference(vec![]);
        if ndt.name == "Node0000"
            && let Some(DataType::Struct(strct)) = &mut ndt.ty
        {
            let mut field = Field::new(Primitive::str.into());
            field
                .attributes
                .insert("serde:field:has_serialize_with", true);
            let late = Struct::named().field("codec", field).build();
            let DataType::Struct(replacement) = Struct::named()
                .field("cycle", Field::new(reference.into()))
                .field("late", Field::new(late))
                .build()
            else {
                unreachable!()
            };
            strct.fields = replacement.fields;
        }
    });
}

#[test]
fn successful_batches_do_not_hide_later_errors_or_cycle_ancestor_errors() {
    let mut types = graph(16, true);
    specta_serde::Format
        .map_types(&types)
        .expect("the initial graph is valid");
    add_invalid_codec(&mut types);
    for format in [
        &specta_serde::Format as &dyn Format,
        &specta_serde::PhasesFormat,
    ] {
        let error = format
            .map_types(&types)
            .expect_err("the nested codec needs an override")
            .to_string();
        insta::allow_duplicates! {
            insta::assert_snapshot!(error, @"Unsupported serde attribute at 'Node0000.late.codec': #[serde(serialize_with)] changes the wire type. Add #[specta(type = ...)] (or #[specta(type = specta_serde::Phased<Serialize, Deserialize>)])");
        }
    }
}

#[derive(Type, serde::Serialize)]
#[specta(collect = false)]
struct Flatten<T> {
    #[serde(flatten)]
    payload: T,
}

#[derive(Type)]
#[specta(collect = false)]
struct Outer<T> {
    inner: Flatten<T>,
}

#[derive(Type)]
#[specta(collect = false)]
struct Object {
    value: String,
}

#[test]
fn concrete_generic_roots_are_checked_independently_in_both_orders() {
    for bad_first in [false, true] {
        let types = if bad_first {
            Types::default()
                .register::<Outer<Vec<u8>>>()
                .register::<Outer<Object>>()
        } else {
            Types::default()
                .register::<Outer<Object>>()
                .register::<Outer<Vec<u8>>>()
        };
        for format in [
            &specta_serde::Format as &dyn Format,
            &specta_serde::PhasesFormat,
        ] {
            let error = format
                .map_types(&types)
                .expect_err("the vector instantiation cannot flatten")
                .to_string();
            assert!(error.contains("Flatten.payload"), "{error}");
            assert!(error.contains("flatten"), "{error}");
        }
    }
}

#[derive(Type, serde::Serialize)]
#[specta(collect = false)]
struct SerializeOnly<T> {
    #[serde(skip_deserializing)]
    inner: Flatten<T>,
}

#[test]
fn phase_dead_generic_visit_does_not_hide_a_later_live_root() {
    type Wire = specta_serde::Phased<Object, Vec<u8>>;
    let valid = Types::default().register::<SerializeOnly<Wire>>();
    specta_serde::PhasesFormat
        .map_types(&valid)
        .expect("only the object-shaped phase is live");
    for bad_first in [false, true] {
        let types = if bad_first {
            Types::default()
                .register::<Flatten<Wire>>()
                .register::<SerializeOnly<Wire>>()
        } else {
            valid.clone().register::<Flatten<Wire>>()
        };
        let error = specta_serde::PhasesFormat
            .map_types(&types)
            .expect_err("the additional root makes the vector-shaped phase live")
            .to_string();
        assert!(error.contains("Flatten.payload"), "{error}");
        assert!(error.contains("flatten"), "{error}");
    }
}

#[test]
fn field_key_fast_path_does_not_skip_metadata_or_container_rules() {
    for container_rule in [false, true] {
        let mut types = graph(4, false);
        types.for_each_named_mut(|ndt| {
            if ndt.name == "Node0000"
                && let Some(DataType::Struct(strct)) = &mut ndt.ty
            {
                if container_rule {
                    strct.attributes.insert(
                        "serde:container:rename_all_serialize",
                        "UPPERCASE".to_string(),
                    );
                } else if let specta::datatype::Fields::Named(fields) = &mut strct.fields {
                    fields.fields[0]
                        .1
                        .attributes
                        .insert("serde:field:rename_serialize", "renamed".to_string());
                }
            }
        });
        let error = specta_serde::Format
            .map_types(&types)
            .expect_err("the renamed key differs between directions")
            .to_string();
        assert!(error.contains("Node0000.next"), "{error}");
        assert!(error.contains("rename"), "{error}");
    }
}
