use specta::Type;
use specta_typescript::{legacy::ExportPath, BigIntExportBehavior, Error, Typescript};

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
    // TODO: Fix errors
    for_bigint_types!(T -> |name| assert_eq!(crate::ts::inline::<T>(&Typescript::default()).map_err(|e| e.to_string()), Err("Attempted to export \"\" but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!\n".into())));
    for_bigint_types!(T -> |name| assert_eq!(crate::ts::inline::<T>(&Typescript::new()).map_err(|e| e.to_string()), Err("Attempted to export \"\" but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!\n".into())));
    for_bigint_types!(T -> |name| assert_eq!(crate::ts::inline::<T>(&Typescript::new().bigint(BigIntExportBehavior::Fail)).map_err(|e| e.to_string()), Err("Attempted to export \"\" but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!\n".into())));

    for_bigint_types!(T -> |name| assert_eq!(crate::ts::inline::<T>(&Typescript::new().bigint(BigIntExportBehavior::String)).map_err(|e| e.to_string()), Ok("string".into())));
    for_bigint_types!(T -> |name| assert_eq!(crate::ts::inline::<T>(&Typescript::new().bigint(BigIntExportBehavior::Number)).map_err(|e| e.to_string()), Ok("number".into())));
    for_bigint_types!(T -> |name| assert_eq!(crate::ts::inline::<T>(&Typescript::new().bigint(BigIntExportBehavior::BigInt)).map_err(|e| e.to_string()), Ok("bigint".into())));

    // // // Check error messages are working correctly -> These tests second for `ExportPath` which is why they are so comprehensive
    assert_eq!(
        crate::ts::inline::<StructWithBigInt>(&Typescript::default()).map_err(|e| e.to_string()),
        Err("Attempted to export \"StructWithBigInt.a\" but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!\n".into())
    );
    assert_eq!(
        crate::ts::inline::<StructWithStructWithBigInt>(&Typescript::default())
            .map_err(|e| e.to_string()),
        Err("Attempted to export \"StructWithStructWithBigInt.abc.StructWithBigInt.a\" but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!\n".into())
    );
    assert_eq!(
        crate::ts::inline::<StructWithStructWithStructWithBigInt>(&Typescript::default())
            .map_err(|e| e.to_string()),
        Err("Attempted to export \"StructWithStructWithStructWithBigInt.field1.StructWithStructWithBigInt.abc.StructWithBigInt.a\" but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!\n".into())
    );
    assert_eq!(
        crate::ts::inline::<EnumWithStructWithStructWithBigInt>(&Typescript::default())
            .map_err(|e| e.to_string()),
        Err("Attempted to export \"EnumWithStructWithStructWithBigInt.A.StructWithStructWithBigInt.abc.StructWithBigInt.a\" but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!\n".into())
    );
    // TODO: This required `inline` to work better on `Option<T>`
    // assert_eq!(
    //     specta_typescript::legacy::inline::<StructWithOptionWithStructWithBigInt>(&Typescript::default()),
    //     Err(ExportError::BigIntForbidden(ExportPath::new_unsafe(
    //         "StructWithOptionWithStructWithBigInt.optional_field -> StructWithStructWithBigInt.abc -> StructWithBigInt.a -> i128"
    //     )))
    // );
    assert_eq!(
        crate::ts::inline::<EnumWithStructWithStructWithBigInt>(&Typescript::default())
            .map_err(|e| e.to_string()),
        Err("Attempted to export \"EnumWithStructWithStructWithBigInt.A.StructWithStructWithBigInt.abc.StructWithBigInt.a\" but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!\n".into())
    );
    assert_eq!(
        crate::ts::inline::<EnumWithInlineStructWithBigInt>(&Typescript::default())
            .map_err(|e| e.to_string()),
        Err("Attempted to export \"EnumWithInlineStructWithBigInt.B.a\" but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!\n".into())
    );
}
