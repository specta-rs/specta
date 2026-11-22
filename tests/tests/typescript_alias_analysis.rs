use specta::{
    Types,
    datatype::{DataType, Enum, Field, NamedDataType, NamedReference, Primitive, Variant},
};
use specta_typescript::{JSDoc, Typescript, primitives};

fn union(branches: impl IntoIterator<Item = DataType>) -> DataType {
    let mut enm = Enum::default();
    enm.variants = branches
        .into_iter()
        .enumerate()
        .map(|(i, ty)| {
            (
                format!("Branch{i}").into(),
                Variant::unnamed().field(Field::new(ty)).build(),
            )
        })
        .collect();
    enm.into()
}

fn graph(
    count: usize,
    edges: impl Fn(usize, &[NamedReference]) -> Vec<DataType>,
) -> (Types, Vec<NamedReference>) {
    let mut types = Types::default();
    let nodes = (0..count)
        .map(|i| {
            NamedDataType::new(format!("Node{i:04}"), &mut types, |_, _, ndt| {
                ndt.module_path = "alias_analysis".into();
                ndt.ty = Some(Primitive::str.into());
            })
        })
        .collect::<Vec<_>>();
    types.for_each_named_mut(|ndt| {
        let index = ndt.name.trim_start_matches("Node").parse().unwrap();
        ndt.ty = Some(union(edges(index, &nodes)));
    });
    (types, nodes)
}

fn reference(handle: &NamedReference) -> DataType {
    handle.reference(vec![]).into()
}

#[test]
fn long_acyclic_alias_chain_preserves_every_reference_in_both_export_orders() {
    let (types, _) = graph(128, |i, nodes| {
        vec![
            nodes
                .get(i + 1)
                .map(reference)
                .unwrap_or(Primitive::str.into()),
            Primitive::bool.into(),
        ]
    });
    let expected = (0..128)
        .map(|i| {
            let next = if i == 127 {
                "string".into()
            } else {
                format!("Node{:04}", i + 1)
            };
            format!("export type Node{i:04} = {next} | boolean;\n")
        })
        .collect::<Vec<_>>();
    let nodes = types.iter_sorted().collect::<Vec<_>>();
    assert_eq!(
        primitives::export(&Typescript::default(), &types, nodes.iter().copied(), "").unwrap(),
        expected.join("\n")
    );
    assert_eq!(
        primitives::export(
            &Typescript::default(),
            &types,
            nodes.iter().rev().copied(),
            ""
        )
        .unwrap(),
        expected.into_iter().rev().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn shared_acyclic_tail_and_duplicate_edges_remain_uncollapsed() {
    let (types, _) = graph(5, |i, nodes| match i {
        0 => vec![reference(&nodes[1]), reference(&nodes[2])],
        1 => vec![reference(&nodes[3]), reference(&nodes[3])],
        2 => vec![reference(&nodes[3]), reference(&nodes[4])],
        3 => vec![reference(&nodes[4]), Primitive::bool.into()],
        _ => vec![Primitive::str.into()],
    });
    insta::assert_snapshot!(
        primitives::export(&Typescript::default(), &types, types.iter_sorted(), "").unwrap(),
        @r"
    export type Node0000 = Node0001 | Node0002;

    export type Node0001 = Node0003;

    export type Node0002 = Node0003 | Node0004;

    export type Node0003 = Node0004 | boolean;

    export type Node0004 = string;
    "
    );
}

#[test]
fn cycle_next_to_cached_acyclic_tail_still_collapses_with_null_and_external_references() {
    let (types, _) = graph(5, |i, nodes| match i {
        0 => vec![reference(&nodes[1]), reference(&nodes[3])],
        1 => vec![
            DataType::Nullable(Box::new(reference(&nodes[2]))),
            reference(&nodes[3]),
        ],
        2 => vec![reference(&nodes[1]), Primitive::bool.into()],
        3 => vec![reference(&nodes[4])],
        _ => vec![Primitive::str.into()],
    });
    // Export the acyclic tail first to populate the proof cache before the
    // predecessor and cyclic component are encountered.
    let nodes = types.iter_sorted().collect::<Vec<_>>();
    let order = [nodes[3], nodes[4], nodes[0], nodes[1], nodes[2]];
    insta::assert_snapshot!(
        primitives::export(&Typescript::default(), &types, order.into_iter(), "").unwrap(),
        @r"
    export type Node0003 = Node0004;

    export type Node0004 = string;

    export type Node0000 = Node0001 | Node0003;

    export type Node0001 = Node0003 | boolean | null;

    export type Node0002 = boolean | Node0003 | null;
    "
    );
    let jsdoc = primitives::export(&JSDoc::default(), &types, order.into_iter(), "").unwrap();
    assert!(
        jsdoc.contains("@typedef {Node0003 | boolean | null} Node0001"),
        "{jsdoc}"
    );
    assert!(
        jsdoc.contains("@typedef {boolean | Node0003 | null} Node0002"),
        "{jsdoc}"
    );
}

#[test]
fn alias_proofs_do_not_survive_between_exports_or_conflate_cloned_declarations() {
    let (types, nodes) = graph(2, |i, nodes| {
        if i == 0 {
            vec![reference(&nodes[1])]
        } else {
            vec![Primitive::str.into()]
        }
    });
    let mut altered = types
        .get(&nodes[0])
        .expect("Node0000 is registered")
        .clone();
    altered.ty = Some(union([reference(&nodes[0]), Primitive::bool.into()]));
    let output = primitives::export(
        &Typescript::default(),
        &types,
        [
            types.get(&nodes[0]).expect("Node0000 is registered"),
            &altered,
        ]
        .into_iter(),
        "",
    )
    .unwrap();
    assert!(
        output.ends_with("export type Node0000 = boolean;\n"),
        "{output}"
    );

    let mut altered_tail = types
        .get(&nodes[1])
        .expect("Node0001 is registered")
        .clone();
    altered_tail.ty = Some(union([reference(&nodes[0]), Primitive::bool.into()]));
    let output = primitives::export(
        &Typescript::default(),
        &types,
        [
            types.get(&nodes[0]).expect("Node0000 is registered"),
            &altered_tail,
        ]
        .into_iter(),
        "",
    )
    .unwrap();
    assert!(
        output.ends_with("export type Node0001 = boolean;\n"),
        "a cached tail must not hide a name-based back edge to an altered root: {output}"
    );

    let (changed, _) = graph(1, |_, nodes| {
        vec![reference(&nodes[0]), Primitive::bool.into()]
    });
    assert_eq!(
        primitives::export(&Typescript::default(), &changed, changed.iter_sorted(), "").unwrap(),
        "export type Node0000 = boolean;\n"
    );
}
