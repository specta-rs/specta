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

// --- lifetime bounds on generic parameters ---
//
// `build` is a nested fn and cannot name the container's lifetime parameters, so
// lifetime predicates must not be copied onto it.
#[derive(Type)]
#[specta(collect = false)]
pub struct LifetimeOutlives<'a, T>
where
    'a: 'static,
{
    a: &'a T,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct ParamOutlivesLifetime<'a, T>
where
    T: 'a,
{
    a: &'a T,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct InlineLifetimeBound<'a, T: 'a> {
    a: &'a T,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct LifetimeAndTraitBound<'a, T: Trait + 'a> {
    a: &'a Wrapper<T>,
}

// --- `#[specta(bound = "...")]` combined with a field requiring that bound ---
#[derive(Type)]
#[specta(collect = false)]
pub struct Wrapper2<T: Trait, U>(T, U);

#[derive(Type)]
#[specta(bound = "T: Trait + Type", collect = false)]
pub struct CustomBoundNested<T: Trait> {
    a: Wrapper2<T, T>,
}

// --- a `macro!`-typed field mentioning a generic parameter ---
macro_rules! ty_of {
    ($t:ty) => {
        $t
    };
}

#[derive(Type)]
#[specta(collect = false)]
pub struct MacroTypedField<T> {
    a: ty_of!(T),
}

// --- two bounded parameters sharing a concrete type ---
#[derive(Type)]
#[specta(collect = false)]
pub struct TwoParamsReversed<K: Trait, V: Trait> {
    map: HashMap<V, K>,
}

// --- `FixedArray` must route its element through `Types::define` ---
#[derive(Type)]
#[specta(collect = false)]
pub struct FixedArrayHolder<T: Trait + Type> {
    a: specta_util::FixedArray<3, T>,
}

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
fn lifetime_bounds_on_generic_parameters_compile() {
    let mut types = specta::Types::default();
    let _ = LifetimeOutlives::<'_, Impl>::definition(&mut types);
    let _ = ParamOutlivesLifetime::<'_, Impl>::definition(&mut types);
    let _ = InlineLifetimeBound::<'_, Impl>::definition(&mut types);
    let _ = LifetimeAndTraitBound::<'_, Impl>::definition(&mut types);
}

#[test]
fn custom_bound_with_bounded_field_compiles() {
    let mut types = specta::Types::default();
    let _ = CustomBoundNested::<Impl>::definition(&mut types);
}

#[test]
fn macro_typed_field_is_not_baked() {
    let mut types = specta::Types::default();
    let _ = MacroTypedField::<Impl>::definition(&mut types);

    let ty = find_exported(&types, "MacroTypedField");
    assert!(
        matches!(field_ty(ty, "a"), DataType::Generic(_)),
        "a `macro!`-typed field baked the first instantiation's concrete type: {:?}",
        field_ty(ty, "a")
    );
}

#[test]
fn fixed_array_element_is_generic() {
    let mut types = specta::Types::default();
    let _ = FixedArrayHolder::<Impl>::definition(&mut types);

    let ty = find_exported(&types, "FixedArrayHolder");
    let DataType::List(list) = field_ty(ty, "a") else {
        panic!("expected list, got {:?}", field_ty(ty, "a"));
    };
    assert_eq!(
        generic_name(&list.ty).map(|g| g.name().to_string()),
        Some("T".to_string())
    );
}

/// KNOWN LIMITATION (tracked in #591): when two *bounded* parameters share the
/// same concrete type, the declaration-time substitution is keyed by that
/// concrete type and cannot tell the parameters apart, so both arguments resolve
/// to the same `Generic`. Unbounded parameters (and every field whose parameter
/// has no declared bounds) use the precise placeholder path and are unaffected.
#[test]
fn repeated_concrete_type_for_multiple_parameters_is_a_known_limitation() {
    let mut types = specta::Types::default();
    let _ = GenericMap::<Impl, Impl>::definition(&mut types);
    let ty = find_exported(&types, "GenericMap");
    let DataType::Map(map) = unwrap_inline(field_ty(ty, "map")) else {
        panic!("expected map");
    };
    assert_eq!(
        generic_name(map.key_ty()).map(|g| g.name().to_string()),
        Some("V".to_string())
    );
    assert_eq!(
        generic_name(map.value_ty()).map(|g| g.name().to_string()),
        Some("V".to_string())
    );

    // The reversed argument order behaves the same way.
    let mut types = specta::Types::default();
    let _ = TwoParamsReversed::<Impl, Impl>::definition(&mut types);
    let ty = find_exported(&types, "TwoParamsReversed");
    let DataType::Map(map) = unwrap_inline(field_ty(ty, "map")) else {
        panic!("expected map");
    };
    assert_eq!(
        generic_name(map.key_ty()).map(|g| g.name().to_string()),
        Some("V".to_string())
    );
    assert_eq!(
        generic_name(map.value_ty()).map(|g| g.name().to_string()),
        Some("V".to_string())
    );

    // Concrete instantiation is still correct regardless.
    let _ = A::<Impl>::definition(&mut types);
}

fn find_exported<'a>(types: &'a specta::Types, name: &str) -> &'a DataType {
    types
        .into_sorted_iter()
        .find(|ndt| ndt.name == name)
        .and_then(|ndt| ndt.ty.as_ref())
        .unwrap_or_else(|| panic!("{name} not exported"))
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
