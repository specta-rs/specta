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
        types.named_len(),
        2,
        "distinct declarations must not share a graph node"
    );
    assert_ne!(
        bool_ty, u32_ty,
        "distinct declarations must not reference one another"
    );
}

/// A declaration's identity is stable across copies of the graph: the same
/// reference still names the same declaration in a clone.
#[test]
fn named_type_identity_survives_graph_copies() {
    use specta::datatype::{DataType, Reference};

    let mut types = Types::default();
    let DataType::Reference(Reference::Named(original)) =
        <IdentityItem as Type>::definition(&mut types)
    else {
        unreachable!("IdentityItem is a named type");
    };
    let DataType::Reference(Reference::Named(copied)) =
        <IdentityItem as Type>::definition(&mut types.clone())
    else {
        unreachable!("IdentityItem is a named type");
    };

    assert_eq!(original, copied);

    // A different declaration is a different identity.
    let other = specta::datatype::NamedDataType::new("Other", &mut types.clone(), |_, _, ndt| {
        ndt.ty = Some(bool::definition(&mut Types::default()));
    });
    assert_ne!(original, other);
}

/// Editing a node only changes the value stored under its key. The identity
/// is the key and is not part of the value, so even replacing the whole
/// value cannot redirect a reference to another declaration.
#[test]
fn editing_a_named_type_keeps_its_identity() {
    use specta::datatype::{NamedDataType, Primitive};

    let mut types = Types::default();
    let a = NamedDataType::new("A", &mut types, |_, _, ndt| {
        ndt.ty = Some(Primitive::bool.into());
    });
    let b = NamedDataType::new("B", &mut types, |_, _, ndt| {
        ndt.ty = Some(Primitive::u32.into());
    });

    assert_ne!(a, b, "different declarations have different identities");

    // In-place edits touch only the targeted entry.
    types.get_mut(&a).expect("A is registered").docs = "edited".into();
    assert_eq!(types.get(&a).expect("A is registered").docs, "edited");
    assert_eq!(types.get(&b).expect("B is registered").docs, "");

    // Replacing the whole value replaces the contents only: there is no
    // identity in the value to disagree with the key, so nothing needs
    // repairing and both entries keep resolving.
    *types.get_mut(&a).expect("A is registered") = types.get(&b).unwrap().clone();

    let edited = types.get(&a).expect("A is still registered");
    assert_eq!(edited.name, "B", "the assignment replaced the contents");
    assert_eq!(
        types.get(&b).map(|ndt| ndt.name.as_ref()),
        Some("B"),
        "B is unchanged"
    );
    assert_eq!(types.named_len(), 2, "the entries did not merge");
    assert_ne!(a, b, "the declarations stay distinct");
}

#[derive(Type)]
#[specta(collect = false)]
struct IdentityItem {
    value: bool,
}
