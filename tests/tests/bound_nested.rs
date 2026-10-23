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

pub trait Trait2 {}

impl Trait2 for Impl {}

// A user trait literally named `Type`, in another module.
pub mod other {
    pub trait Type {}
}

impl other::Type for Impl {}

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

// A `where` predicate that bundles a lifetime bound with non-lifetime bounds.
// Only the lifetime part may be dropped: dropping the whole predicate loses the
// `Trait` bound `build`'s body needs, and the field type fails to type-check.
#[derive(Type)]
#[specta(collect = false)]
pub struct MixedWhere<'a, T>
where
    T: 'a + Trait,
{
    a: &'a Wrapper<T>,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct MixedWhereTwo<'a, T>
where
    T: 'a + Trait + Trait2,
{
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

// --- three bounded parameters sharing a concrete type ---
#[derive(Type)]
#[specta(collect = false)]
pub struct ThreeParams<K: Trait, V: Trait, W: Trait> {
    map: HashMap<K, HashMap<V, W>>,
}

// --- a parameter mentioned only by a skipped field ---
//
// `A` is mentioned by `SkipMid`'s `#[specta(skip)]`ed field, so `SkipMid` never
// resolves it. The enclosing substitution scope must still leave `B` mapped to
// the argument that sits at `B`'s position (`V`), not the one at `A`'s.
#[derive(Type)]
#[specta(collect = false)]
pub struct SkipMid<A: Trait + Type, B: Trait + Type> {
    #[specta(skip)]
    a: std::marker::PhantomData<A>,
    b: B,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct SkipTop<K: Trait + Type, V: Trait + Type> {
    m: SkipMid<K, V>,
}

// The skipped parameter's argument is itself a compound, so it occupies more
// than one occurrence of the enclosing parameters.
pub struct Compound<A, B>(std::marker::PhantomData<(A, B)>);

impl<A: Trait, B: Trait> Trait for Compound<A, B> {}

impl<A: Type, B: Type> Type for Compound<A, B> {
    fn definition(types: &mut specta::Types) -> DataType {
        let a = types.define::<A>();
        let b = types.define::<B>();
        DataType::Tuple(specta::datatype::Tuple::new(vec![a, b]))
    }
}

#[derive(Type)]
#[specta(collect = false)]
pub struct SkipCompoundMid<A: Trait + Type, B: Trait + Type> {
    #[specta(skip)]
    a: std::marker::PhantomData<A>,
    b: B,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct SkipCompoundTop<K: Trait + Type, V: Trait + Type, W: Trait + Type> {
    m: SkipCompoundMid<Compound<K, V>, W>,
}

// --- a manual impl that resolves its own arguments out of order ---
//
// `define` must search only the innermost scope, or an enclosing type's
// parameters leak into this type's own.
pub struct ReversedPair<K, V>(std::marker::PhantomData<(K, V)>);

impl<K: Type, V: Type> Type for ReversedPair<K, V> {
    fn definition(types: &mut specta::Types) -> DataType {
        let v = types.define::<V>();
        let k = types.define::<K>();
        DataType::Tuple(specta::datatype::Tuple::new(vec![v, k]))
    }
}

#[derive(Type)]
#[specta(collect = false)]
pub struct ReversedPairHolder<K: Trait, V: Trait> {
    p: ReversedPair<K, V>,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct Mid<A: Trait, B: Trait> {
    p: ReversedPair<A, B>,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct MidHolder<K: Trait, V: Trait> {
    m: Mid<K, V>,
}

// --- a concrete field inside a nested generic ---
//
// `items` mentions no enclosing parameter, so it must resolve concretely even
// when `Page` is built while resolving `Response<Item>` — the enclosing
// substitution scope must not be reachable from a field that has none of its
// own.
#[derive(Type)]
#[specta(collect = false)]
pub struct Item;

impl Trait for Item {}

#[derive(Type)]
#[specta(collect = false)]
pub struct Page<T: Trait> {
    items: Vec<Item>,
    t: T,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct Response<T: Trait> {
    page: Page<T>,
}

// --- `FixedArray` must route its element through `Types::define` ---
#[derive(Type)]
#[specta(collect = false)]
pub struct FixedArrayHolder<T: Trait + Type> {
    a: specta_util::FixedArray<3, T>,
}

// --- a user trait literally named `Type`, in another module ---
//
// `build`'s automatic `Type` predicate only replaces specta's own `Type` bound.
// Matching on the last path segment would mistake `other::Type` for it and drop
// a bound the field's own declaration requires.
#[derive(Type)]
#[specta(collect = false)]
pub struct Wrapper3<T: other::Type>(T);

#[derive(Type)]
#[specta(collect = false)]
pub struct UsesWrapper3<T: other::Type> {
    a: Wrapper3<T>,
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
fn mixed_where_predicate_keeps_non_lifetime_bounds() {
    let mut types = specta::Types::default();
    let _ = MixedWhere::<'_, Impl>::definition(&mut types);
    let _ = MixedWhereTwo::<'_, Impl>::definition(&mut types);
}

#[test]
fn user_trait_named_type_is_not_dropped() {
    let mut types = specta::Types::default();
    let _ = UsesWrapper3::<Impl>::definition(&mut types);
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

/// When two *bounded* parameters share the same concrete type the substitution
/// entries are consumed positionally, so each parameter keeps its own `Generic`
/// name. See <https://github.com/specta-rs/specta/issues/609>.
#[test]
fn repeated_concrete_type_for_multiple_parameters_keeps_distinct_generics() {
    let mut types = specta::Types::default();
    let _ = GenericMap::<Impl, Impl>::definition(&mut types);
    let ty = find_exported(&types, "GenericMap");
    let DataType::Map(map) = unwrap_inline(field_ty(ty, "map")) else {
        panic!("expected map");
    };
    assert_eq!(
        generic_name(map.key_ty()).map(|g| g.name().to_string()),
        Some("K".to_string())
    );
    assert_eq!(
        generic_name(map.value_ty()).map(|g| g.name().to_string()),
        Some("V".to_string())
    );

    // The reversed argument order keeps its own order too.
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
        Some("K".to_string())
    );

    // Concrete instantiation is still correct regardless.
    let mut types = specta::Types::default();
    let concrete = GenericMap::<Impl, Impl>::definition(&mut types);
    assert!(
        !contains_generic(&concrete),
        "concrete instantiation leaked a Generic node: {concrete:?}"
    );
    let _ = A::<Impl>::definition(&mut types);
}

#[test]
fn three_repeated_concrete_types_keep_distinct_generics() {
    let mut types = specta::Types::default();
    let _ = ThreeParams::<Impl, Impl, Impl>::definition(&mut types);
    let ty = find_exported(&types, "ThreeParams");

    let DataType::Map(map) = unwrap_inline(field_ty(ty, "map")) else {
        panic!("expected map");
    };
    assert_eq!(
        generic_name(map.key_ty()).map(|g| g.name().to_string()),
        Some("K".to_string())
    );

    let DataType::Map(inner) = unwrap_inline(map.value_ty()) else {
        panic!("expected nested map");
    };
    assert_eq!(
        generic_name(inner.key_ty()).map(|g| g.name().to_string()),
        Some("V".to_string())
    );
    assert_eq!(
        generic_name(inner.value_ty()).map(|g| g.name().to_string()),
        Some("W".to_string())
    );
}

#[test]
fn parameter_skipped_by_nested_type_keeps_its_own_slot() {
    let mut types = specta::Types::default();
    let _ = SkipTop::<Impl, Impl>::definition(&mut types);

    let ty = find_exported(&types, "SkipTop");
    let generics = named_generics(field_ty(ty, "m")).expect("`m` should reference SkipMid");
    assert_eq!(generics.len(), 1, "SkipMid only resolves `B`: {generics:?}");
    assert_eq!(
        generic_name(&generics[0].1).map(|g| g.name().to_string()),
        Some("V".to_string()),
        "the skipped `A` shifted the slot: {generics:?}"
    );
}

#[test]
fn skipped_compound_argument_does_not_shift_later_slots() {
    let mut types = specta::Types::default();
    let _ = SkipCompoundTop::<Impl, Impl, Impl>::definition(&mut types);

    let ty = find_exported(&types, "SkipCompoundTop");
    let generics = named_generics(field_ty(ty, "m")).expect("`m` should reference SkipCompoundMid");
    assert_eq!(
        generics.len(),
        1,
        "SkipCompoundMid only resolves `B`: {generics:?}"
    );
    assert_eq!(
        generic_name(&generics[0].1).map(|g| g.name().to_string()),
        Some("W".to_string()),
        "the skipped compound `HashMap<K, V>` shifted the slot: {generics:?}"
    );
}

#[test]
fn manual_impl_resolving_arguments_out_of_order_keeps_them_distinct() {
    let mut types = specta::Types::default();
    let _ = ReversedPairHolder::<Impl, OtherImpl>::definition(&mut types);

    let ty = find_exported(&types, "ReversedPairHolder");
    let DataType::Tuple(tuple) = field_ty(ty, "p") else {
        panic!("expected tuple, got {:?}", field_ty(ty, "p"));
    };
    assert_eq!(
        generic_name(&tuple.elements[0]).map(|g| g.name().to_string()),
        Some("V".to_string()),
        "{tuple:?}"
    );
    assert_eq!(
        generic_name(&tuple.elements[1]).map(|g| g.name().to_string()),
        Some("K".to_string()),
        "{tuple:?}"
    );
}

#[test]
fn enclosing_parameters_do_not_leak_into_a_nested_declaration() {
    let mut types = specta::Types::default();
    let _ = MidHolder::<Impl, OtherImpl>::definition(&mut types);

    // `Mid`'s own declaration must name its own parameters, never the enclosing
    // `K`/`V`.
    let mid = find_exported(&types, "Mid");
    let DataType::Tuple(tuple) = field_ty(mid, "p") else {
        panic!("expected tuple, got {:?}", field_ty(mid, "p"));
    };
    assert_eq!(
        generic_name(&tuple.elements[0]).map(|g| g.name().to_string()),
        Some("B".to_string()),
        "enclosing parameters leaked into Mid: {tuple:?}"
    );
    assert_eq!(
        generic_name(&tuple.elements[1]).map(|g| g.name().to_string()),
        Some("A".to_string()),
        "enclosing parameters leaked into Mid: {tuple:?}"
    );
}

fn assert_concrete_items(types: &specta::Types, context: &str) {
    let page = types
        .into_sorted_iter()
        .find(|ndt| ndt.name == "Page")
        .and_then(|ndt| ndt.ty.as_ref())
        .unwrap_or_else(|| panic!("{context}: Page not exported"));

    let items = field_ty(page, "items");
    assert!(
        !contains_generic(items),
        "{context}: `items` baked an enclosing parameter: {items:?}"
    );
}

#[test]
fn concrete_field_does_not_bake_an_enclosing_parameter() {
    let mut types = specta::Types::default();
    let _ = Response::<Item>::definition(&mut types);

    assert_concrete_items(&types, "through Response<Item>");
}

#[test]
fn concrete_field_is_stable_regardless_of_resolution_order() {
    // `Page`'s declaration is registered once, so whichever of `Page`/`Response`
    // is resolved first must not change what the other sees.
    let mut page_first = specta::Types::default();
    let _ = Page::<Item>::definition(&mut page_first);
    let _ = Response::<Item>::definition(&mut page_first);
    assert_concrete_items(&page_first, "Page first");

    let mut response_first = specta::Types::default();
    let _ = Response::<Item>::definition(&mut response_first);
    let _ = Page::<Item>::definition(&mut response_first);
    assert_concrete_items(&response_first, "Response first");
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
