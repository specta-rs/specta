#![allow(dead_code)]

use specta::Type;

// Test 1: Add extra bounds in addition to Type
#[derive(Type)]
#[specta(bound = "T: Clone + Type", collect = false)]
struct CustomBound<T> {
    value: T,
}

// Test 2: Multiple type parameters with custom bounds
#[derive(Type)]
#[specta(bound = "T: Clone + Type, U: std::fmt::Debug + Type", collect = false)]
struct MultiBound<T, U> {
    t: T,
    u: U,
}

// Test 3: Complex trait bounds  
#[derive(Type)]
#[specta(bound = "T: Clone + std::fmt::Debug + Type", collect = false)]
struct ComplexBound<T> {
    value: T,
}

// Test 4: Struct with existing where clause - merges with custom bound
#[derive(Type)]
#[specta(bound = "T: Type", collect = false)]
struct ExistingWhere<T>
where
    T: Clone,
{
    value: T,
}

// Test 5: Enum with custom bound
#[derive(Type)]
#[specta(bound = "T: Clone + Type", collect = false)]
enum EnumWithBound<T> {
    Variant(T),
    Other,
}

// Test 6: Lifetime bounds combined with type bounds
#[derive(Type)]
#[specta(bound = "T: Type + 'static", collect = false)]
struct LifetimeBound<T> {
    value: T,
}

// Test 7: Demonstrate the bound is actually checked
#[derive(Type)]
#[specta(bound = "T: Clone + Type", collect = false)]
struct RequiresClone<T> {
    value: T,
}

#[test]
fn test_custom_bound() {
    // CustomBound should compile with Clone + Type bound
    #[derive(Clone, Type)]
    #[specta(collect = false)]
    struct CloneAndType;
    
    let _: CustomBound<CloneAndType> = CustomBound {
        value: CloneAndType,
    };
}

#[test]
fn test_multi_bound() {
    #[derive(Clone, Debug, Type)]
    #[specta(collect = false)]
    struct AllTraits;
    
    let _: MultiBound<AllTraits, AllTraits> = MultiBound {
        t: AllTraits,
        u: AllTraits,
    };
}

#[test]
fn test_complex_bound() {
    #[derive(Clone, Debug, Type)]
    #[specta(collect = false)]
    struct AllTraits;
    
    let _: ComplexBound<AllTraits> = ComplexBound {
        value: AllTraits,
    };
}

#[test]
fn test_existing_where() {
    #[derive(Clone, Type)]
    #[specta(collect = false)]
    struct BothTraits;
    
    let _: ExistingWhere<BothTraits> = ExistingWhere {
        value: BothTraits,
    };
}

#[test]
fn test_enum_bound() {
    #[derive(Clone, Type)]
    #[specta(collect = false)]
    struct CloneAndType;
    
    let _: EnumWithBound<CloneAndType> = EnumWithBound::Other;
}
