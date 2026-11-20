//! Declaration identity.
//!
//! A declaration's identity must be nominal. Derive-generated code can be
//! expanded twice by a `macro_rules!` macro with the same type name, module path
//! and source span, and those expansions are distinct Rust types.

use specta::{Type, Types};

/// Two expansions of the same derive are distinct declarations, so they must be
/// distinct graph nodes instead of one node whose first definition wins.
#[test]
fn macro_expanded_declarations_have_distinct_identity() {
    macro_rules! scoped {
        ($field:ty) => {{
            #[derive(Type)]
            #[specta(collect = false)]
            struct Local {
                x: $field,
            }
            <Local as Type>::definition
        }};
    }

    let mut types = Types::default();
    let bool_ty = scoped!(bool)(&mut types);
    let u32_ty = scoped!(u32)(&mut types);

    assert_eq!(
        types.len(),
        2,
        "distinct declarations must not share a graph node"
    );
    assert_ne!(
        bool_ty, u32_ty,
        "distinct declarations must not reference one another"
    );
}

/// A declaration's identity is stable across copies of the graph and usable as
/// a map key.
#[test]
fn named_type_ids_survive_graph_copies() {
    let types = Types::default().register::<IdentityItem>();
    let clone = types.clone();

    let id = types
        .into_sorted_iter()
        .next()
        .expect("IdentityItem is registered")
        .id()
        .clone();
    let clone_id = clone
        .into_sorted_iter()
        .next()
        .expect("IdentityItem is registered")
        .id()
        .clone();

    assert_eq!(id, clone_id);
    assert_ne!(
        id,
        specta::datatype::NamedDataType::new("Other", &mut types.clone(), |_, ndt| {
            ndt.ty = Some(bool::definition(&mut Types::default()));
        })
        .id()
        .clone()
    );
}

/// Replacing a node's contents must not redirect references to it at another
/// declaration. The collection deliberately does not hand out a mutable
/// reference to a whole node.
#[test]
fn editing_a_named_type_keeps_its_identity() {
    use specta::datatype::{NamedDataType, Primitive, Reference};

    let mut types = Types::default();
    let a = NamedDataType::new("A", &mut types, |_, ndt| {
        ndt.ty = Some(Primitive::bool.into());
    });
    let b = NamedDataType::new("B", &mut types, |_, ndt| {
        ndt.ty = Some(Primitive::u32.into());
    });

    let (Reference::Named(a_ref), Reference::Named(b_ref)) =
        (a.reference(vec![]), b.reference(vec![]))
    else {
        unreachable!("named datatypes produce named references");
    };

    assert!(types.edit(&a_ref, |ndt| *ndt = b.clone()));

    let edited = types.get(&a_ref).expect("A is still registered");
    assert_eq!(edited.name, "B", "the edit replaced the contents");
    assert_eq!(edited.id(), a.id(), "the edit kept A's identity");
    assert_eq!(
        types.get(&b_ref).map(|ndt| ndt.id()),
        Some(b.id()),
        "B is unchanged"
    );
}

#[derive(Type)]
#[specta(collect = false)]
struct IdentityItem {
    value: bool,
}
