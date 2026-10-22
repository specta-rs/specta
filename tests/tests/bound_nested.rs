//! Regression tests for <https://github.com/specta-rs/specta/issues/591>.
//!
//! A field whose type declaration itself requires a user trait (e.g.
//! `Wrapper<T: Trait>` used as `Wrapper<T>` inside `A<T: Trait>`) used to make
//! the generated `build` fn fail with `E0277`, because the canonical
//! declaration was resolved through placeholder types that cannot implement
//! arbitrary user traits.
#![allow(clippy::multiple_bound_locations)]

use specta::Type;
use specta::datatype::{DataType, Fields, Generic, NamedReferenceType, Reference};
use std::collections::HashMap;

pub trait Trait {}

#[derive(Type)]
#[specta(collect = false)]
pub struct Wrapper<T: Trait>(T);

// The minimal reproduction from the issue.
#[derive(Type)]
#[specta(collect = false)]
pub struct A<T: Trait> {
    a: Wrapper<T>,
}

// Inline bound form with a nested bounded wrapper.
#[derive(Type)]
#[specta(collect = false)]
pub struct B<T: Trait> {
    a: A<T>,
    list: Vec<Wrapper<T>>,
}

// `where` clause form of the same bound.
#[derive(Type)]
#[specta(collect = false)]
pub struct WhereBound<T>
where
    T: Trait,
{
    a: Wrapper<T>,
}

// Lifetimes alongside a bounded parameter.
#[derive(Type)]
#[specta(collect = false)]
pub struct WithLifetime<'a, T: Trait> {
    a: &'a Wrapper<T>,
}

// Const parameters alongside a bounded parameter.
#[derive(Type)]
#[specta(collect = false)]
pub struct WithConst<const N: usize, T: Trait> {
    a: Wrapper<T>,
    b: [u8; N],
}

// Repeated/recursive use of the same generic definition.
#[derive(Type)]
#[specta(collect = false)]
pub struct Recursive<T: Trait> {
    a: Wrapper<T>,
    children: Vec<Recursive<T>>,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct GenericMap<K: Trait, V: Trait> {
    map: HashMap<K, V>,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct Impl;

impl Trait for Impl {}

#[derive(Type)]
#[specta(collect = false)]
pub struct OtherImpl;

impl Trait for OtherImpl {}

fn field_ty<'a>(ty: &'a DataType, name: &str) -> &'a DataType {
    let DataType::Struct(s) = ty else {
        panic!("expected struct, got {ty:?}");
    };
    let Fields::Named(named) = &s.fields else {
        panic!("expected named fields, got {s:?}");
    };
    named
        .fields
        .iter()
        .find(|(field, _)| field == name)
        .and_then(|(_, field)| field.ty.as_ref())
        .unwrap_or_else(|| panic!("missing field {name:?}"))
}

fn generic_name(ty: &DataType) -> Option<&Generic> {
    match ty {
        DataType::Generic(generic) => Some(generic),
        _ => None,
    }
}

fn named_generics(ty: &DataType) -> Option<&Vec<(Generic, DataType)>> {
    let DataType::Reference(Reference::Named(reference)) = ty else {
        return None;
    };
    let NamedReferenceType::Reference { generics, .. } = &reference.inner else {
        return None;
    };
    Some(generics)
}

/// Passthrough containers like `Vec`/`HashMap` are wrapped in an inline
/// reference; unwrap it to inspect the underlying datatype.
fn unwrap_inline(ty: &DataType) -> &DataType {
    if let DataType::Reference(Reference::Named(reference)) = ty
        && let NamedReferenceType::Inline { dt, .. } = &reference.inner
    {
        return dt;
    }
    ty
}

#[test]
fn nested_bounded_wrapper_keeps_named_identity_and_generic_argument() {
    let mut types = specta::Types::default();
    let _ = A::<Impl>::definition(&mut types);

    let a = types
        .into_sorted_iter()
        .find(|ndt| ndt.name == "A")
        .expect("A declaration should be registered");
    let a = a.ty.as_ref().expect("A is exported as a named type");

    let field = field_ty(a, "a");
    let generics = named_generics(field).expect("`a` should reference Wrapper");
    assert_eq!(generics.len(), 1);
    assert_eq!(
        generic_name(&generics[0].1).map(Generic::name),
        Some(&"T".into())
    );

    let wrapper = types
        .into_sorted_iter()
        .find(|ndt| ndt.name == "Wrapper")
        .expect("Wrapper declaration should be registered");
    assert!(wrapper.ty.is_some());
}

#[test]
fn generic_argument_is_placeholder_only_in_declaration_position() {
    let mut types = specta::Types::default();
    let concrete = A::<Impl>::definition(&mut types);

    // The concrete instantiation must not contain `Generic` nodes.
    assert!(
        !contains_generic(&concrete),
        "concrete instantiation leaked a Generic node: {concrete:?}"
    );
}

#[test]
fn where_clause_bound_compiles() {
    let mut types = specta::Types::default();
    let _ = WhereBound::<Impl>::definition(&mut types);
}

#[test]
fn lifetime_and_const_parameters_compile() {
    let mut types = specta::Types::default();
    let _ = WithLifetime::<'_, Impl>::definition(&mut types);
    let _ = WithConst::<4, Impl>::definition(&mut types);
}

#[test]
fn recursive_definition_shares_identity() {
    let mut types = specta::Types::default();
    let _ = Recursive::<Impl>::definition(&mut types);

    let recursive = types
        .into_sorted_iter()
        .find(|ndt| ndt.name == "Recursive")
        .expect("Recursive declaration should be registered");
    let recursive = recursive.ty.as_ref().expect("Recursive is exported");

    let children = unwrap_inline(field_ty(recursive, "children"));
    let DataType::List(list) = children else {
        panic!("expected list, got {children:?}");
    };
    let generics = named_generics(&list.ty).expect("`children` should reference Recursive");
    assert_eq!(generics.len(), 1);
    assert_eq!(
        generic_name(&generics[0].1).map(Generic::name),
        Some(&"T".into())
    );
}

#[test]
fn multi_parameter_generics_keep_distinct_placeholders() {
    let mut types = specta::Types::default();
    let _ = GenericMap::<Impl, OtherImpl>::definition(&mut types);

    let map = types
        .into_sorted_iter()
        .find(|ndt| ndt.name == "GenericMap")
        .expect("GenericMap declaration should be registered");
    let map = map.ty.as_ref().expect("GenericMap is exported");

    let field = unwrap_inline(field_ty(map, "map"));
    let DataType::Map(map_ty) = field else {
        panic!("expected map, got {field:?}");
    };
    assert_eq!(
        generic_name(map_ty.key_ty()).map(|g| g.name().to_string()),
        Some("K".to_string())
    );
    assert_eq!(
        generic_name(map_ty.value_ty()).map(|g| g.name().to_string()),
        Some("V".to_string())
    );
}

#[test]
fn repeated_concrete_type_for_multiple_parameters_compiles() {
    let mut types = specta::Types::default();
    let _ = GenericMap::<Impl, Impl>::definition(&mut types);
    let _ = A::<Impl>::definition(&mut types);
}

fn contains_generic(ty: &DataType) -> bool {
    match ty {
        DataType::Generic(_) => true,
        DataType::List(list) => contains_generic(&list.ty),
        DataType::Map(map) => contains_generic(map.key_ty()) || contains_generic(map.value_ty()),
        DataType::Nullable(inner) => contains_generic(inner),
        DataType::Tuple(tuple) => tuple.elements.iter().any(contains_generic),
        DataType::Intersection(types) => types.iter().any(contains_generic),
        DataType::Struct(s) => match &s.fields {
            Fields::Unit => false,
            Fields::Unnamed(unnamed) => unnamed
                .fields
                .iter()
                .filter_map(|f| f.ty.as_ref())
                .any(contains_generic),
            Fields::Named(named) => named
                .fields
                .iter()
                .filter_map(|(_, f)| f.ty.as_ref())
                .any(contains_generic),
        },
        DataType::Enum(e) => e.variants.iter().any(|(_, variant)| match &variant.fields {
            Fields::Unit => false,
            Fields::Unnamed(unnamed) => unnamed
                .fields
                .iter()
                .filter_map(|f| f.ty.as_ref())
                .any(contains_generic),
            Fields::Named(named) => named
                .fields
                .iter()
                .filter_map(|(_, f)| f.ty.as_ref())
                .any(contains_generic),
        }),
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => contains_generic(dt),
            NamedReferenceType::Reference { generics, .. } => {
                generics.iter().any(|(_, ty)| contains_generic(ty))
            }
            NamedReferenceType::Recursive(_) => false,
        },
        DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => false,
    }
}
