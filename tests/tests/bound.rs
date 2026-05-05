use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type)]
#[specta(bound = "T: Clone + Type", collect = false)]
struct CustomBound<T> {
    value: T,
}

#[derive(Type)]
#[specta(bound = "T: Clone + Type, U: std::fmt::Debug + Type", collect = false)]
struct MultiBound<T, U> {
    t: T,
    u: U,
}

#[derive(Type)]
#[specta(bound = "T: Clone + std::fmt::Debug + Type", collect = false)]
struct ComplexBound<T> {
    value: T,
}

#[derive(Type)]
#[specta(bound = "T: Type", collect = false)]
struct ExistingWhere<T>
where
    T: Clone,
{
    value: T,
}

#[derive(Type)]
#[specta(bound = "T: Clone + Type", collect = false)]
enum EnumWithBound<T> {
    Variant(T),
    Other,
}

#[derive(Type)]
#[specta(bound = "T: Type + 'static", collect = false)]
struct LifetimeBound<T> {
    value: T,
}

#[derive(Type)]
#[specta(bound = "T: Clone + Type", collect = false)]
struct RequiresClone<T> {
    value: T,
}

#[test]
fn custom_bound() {
    #[derive(Clone, Type)]
    #[specta(collect = false)]
    struct CloneAndType;

    let _: CustomBound<CloneAndType> = CustomBound {
        value: CloneAndType,
    };
}

#[test]
fn multi_bound() {
    #[derive(Clone, Debug, Type)]
    #[specta(collect = false)]
    struct AllTraits;

    let _: MultiBound<AllTraits, AllTraits> = MultiBound {
        t: AllTraits,
        u: AllTraits,
    };
}

#[test]
fn complex_bound() {
    #[derive(Clone, Debug, Type)]
    #[specta(collect = false)]
    struct AllTraits;

    let _: ComplexBound<AllTraits> = ComplexBound { value: AllTraits };
}

#[test]
fn existing_where() {
    #[derive(Clone, Type)]
    #[specta(collect = false)]
    struct BothTraits;

    let _: ExistingWhere<BothTraits> = ExistingWhere { value: BothTraits };
}

#[test]
fn enum_bound() {
    #[derive(Clone, Type)]
    #[specta(collect = false)]
    struct CloneAndType;

    let _: EnumWithBound<CloneAndType> = EnumWithBound::Other;
}

#[test]
fn lifetime_bound() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct StaticType;

    let _: LifetimeBound<StaticType> = LifetimeBound { value: StaticType };
}

#[test]
fn requires_clone_bound() {
    #[derive(Clone, Type)]
    #[specta(collect = false)]
    struct CloneAndType;

    let _: RequiresClone<CloneAndType> = RequiresClone {
        value: CloneAndType,
    };
}

#[test]
fn associated_type_bound_issue_138() {
    // Regression test for https://github.com/specta-rs/specta/issues/138
    trait MyTrait {
        type A: Type;
    }

    struct AssocIsI32;

    impl MyTrait for AssocIsI32 {
        type A = i32;
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct Demo<T: MyTrait> {
        value: T::A,
    }

    let types = Types::default().register::<Demo<AssocIsI32>>();
    let output = Typescript::default()
        .export(&types, specta_serde::Format)
        .unwrap();
    insta::assert_snapshot!("bound-associated-type-issue-138", output);
}
