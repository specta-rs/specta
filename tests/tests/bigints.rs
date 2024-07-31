use specta::Type;
use specta_typescript::{BigIntExportBehavior, ExportError, ExportPath, Typescript};

macro_rules! for_bigint_types {
    (T -> $s:expr) => {{
        for_bigint_types!(usize, isize, i64, u64, i128, u128; $s);
    }};
    ($($i:ty),+; $s:expr) => {{
        $({
            type T = $i;
            $s(stringify!($i))
        })*
    }};
}

#[derive(Type)]
#[specta(export = false)]
pub struct StructWithBigInt {
    pub a: i128,
}

#[derive(Type)]
#[specta(export = false)]

pub struct StructWithStructWithBigInt {
    #[specta(inline)] // Inline required so reference is not used and error is part of parent
    pub abc: StructWithBigInt,
}

#[derive(Type)]
#[specta(export = false)]

pub struct StructWithStructWithStructWithBigInt {
    #[specta(inline)] // Inline required so reference is not used and error is part of parent
    pub field1: StructWithStructWithBigInt,
}

#[derive(Type)]
#[specta(export = false)]
pub struct StructWithOptionWithStructWithBigInt {
    #[specta(inline)] // Inline required so reference is not used and error is part of parent
    pub optional_field: Option<StructWithBigInt>,
}

#[derive(Type)]
#[specta(export = false)]

pub enum EnumWithStructWithStructWithBigInt {
    #[specta(inline)]
    A(StructWithStructWithBigInt),
}

#[derive(Type)]
#[specta(export = false)]

pub enum EnumWithInlineStructWithBigInt {
    #[specta(inline)]
    B { a: i128 },
}

#[test]
fn test_bigint_types() {
    for_bigint_types!(T -> |name| assert_eq!(specta_typescript::inline::<T>(&Typescript::default()), Err(ExportError::BigIntForbidden(ExportPath::new_unsafe(name)))));
    for_bigint_types!(T -> |name| assert_eq!(specta_typescript::inline::<T>(&Typescript::new()), Err(ExportError::BigIntForbidden(ExportPath::new_unsafe(name)))));
    for_bigint_types!(T -> |name| assert_eq!(specta_typescript::inline::<T>(&Typescript::new().bigint(BigIntExportBehavior::Fail)), Err(ExportError::BigIntForbidden(ExportPath::new_unsafe(name)))));
    for_bigint_types!(T -> |name| assert_eq!(specta_typescript::inline::<T>(&Typescript::new().bigint(BigIntExportBehavior::FailWithReason("some reason"))), Err(ExportError::Other(ExportPath::new_unsafe(name), "some reason".into()))));

    for_bigint_types!(T -> |name| assert_eq!(specta_typescript::inline::<T>(&Typescript::new().bigint(BigIntExportBehavior::String)), Ok("string".into())));
    for_bigint_types!(T -> |name| assert_eq!(specta_typescript::inline::<T>(&Typescript::new().bigint(BigIntExportBehavior::Number)), Ok("number".into())));
    for_bigint_types!(T -> |name| assert_eq!(specta_typescript::inline::<T>(&Typescript::new().bigint(BigIntExportBehavior::BigInt)), Ok("bigint".into())));

    // Check error messages are working correctly -> These tests second for `ExportPath` which is why they are so comprehensive
    assert_eq!(
        specta_typescript::inline::<StructWithBigInt>(&Typescript::default()),
        Err(ExportError::BigIntForbidden(ExportPath::new_unsafe(
            "StructWithBigInt.a -> i128"
        )))
    );
    assert_eq!(
        specta_typescript::inline::<StructWithStructWithBigInt>(&Typescript::default()),
        Err(ExportError::BigIntForbidden(ExportPath::new_unsafe(
            "StructWithStructWithBigInt.abc -> StructWithBigInt.a -> i128"
        )))
    );
    assert_eq!(
        specta_typescript::inline::<StructWithStructWithStructWithBigInt>(&Typescript::default()),
        Err(ExportError::BigIntForbidden(ExportPath::new_unsafe(
            "StructWithStructWithStructWithBigInt.field1 -> StructWithStructWithBigInt.abc -> StructWithBigInt.a -> i128"
        )))
    );
    assert_eq!(
        specta_typescript::inline::<EnumWithStructWithStructWithBigInt>(&Typescript::default()),
        Err(ExportError::BigIntForbidden(ExportPath::new_unsafe(
            "EnumWithStructWithStructWithBigInt::A -> StructWithStructWithBigInt.abc -> StructWithBigInt.a -> i128"
        )))
    );
    // TODO: This required `inline` to work better on `Option<T>`
    // assert_eq!(
    //     specta_typescript::inline::<StructWithOptionWithStructWithBigInt>(&Typescript::default()),
    //     Err(ExportError::BigIntForbidden(ExportPath::new_unsafe(
    //         "StructWithOptionWithStructWithBigInt.optional_field -> StructWithStructWithBigInt.abc -> StructWithBigInt.a -> i128"
    //     )))
    // );
    assert_eq!(
        specta_typescript::inline::<EnumWithStructWithStructWithBigInt>(&Typescript::default()),
        Err(ExportError::BigIntForbidden(ExportPath::new_unsafe(
            "EnumWithStructWithStructWithBigInt::A -> StructWithStructWithBigInt.abc -> StructWithBigInt.a -> i128"
        )))
    );
    assert_eq!(
        specta_typescript::inline::<EnumWithInlineStructWithBigInt>(&Typescript::default()),
        Err(ExportError::BigIntForbidden(ExportPath::new_unsafe(
            "EnumWithInlineStructWithBigInt::B.a -> i128"
        )))
    );
}
