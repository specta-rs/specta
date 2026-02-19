use specta::{Type, TypeCollection};
use specta_typescript::{BigIntExportBehavior, Typescript, primitives};

macro_rules! for_bigint_types {
    (T -> $s:expr) => {{
        for_bigint_types!(usize, isize, i64, u64, i128, u128; $s);
    }};
    ($($i:ty),+; $s:expr) => {{
        $({
            type T = $i;
            $s(stringify!($i));
        })*
    }};
}

#[derive(Type)]
#[specta(collect = false)]
struct StructWithBigInt {
    a: i128,
}

#[derive(Type)]
#[specta(collect = false)]
struct StructWithStructWithBigInt {
    #[specta(inline)]
    abc: StructWithBigInt,
}

#[derive(Type)]
#[specta(collect = false)]
struct StructWithStructWithStructWithBigInt {
    #[specta(inline)]
    field1: StructWithStructWithBigInt,
}

#[derive(Type)]
#[specta(collect = false)]
struct StructWithOptionWithStructWithBigInt {
    #[specta(inline)]
    optional_field: Option<StructWithBigInt>,
}

#[derive(Type)]
#[specta(collect = false)]
enum EnumWithStructWithStructWithBigInt {
    #[specta(inline)]
    A(StructWithStructWithBigInt),
}

#[derive(Type)]
#[specta(collect = false)]
enum EnumWithInlineStructWithBigInt {
    #[specta(inline)]
    B { a: i128 },
}

fn inline_for<T: Type>(ts: &Typescript) -> Result<String, specta_typescript::Error> {
    let mut types = TypeCollection::default();
    let dt = T::definition(&mut types);
    primitives::inline(ts, &types, &dt)
}

#[test]
fn bigint_export_behaviors() {
    for_bigint_types!(T -> |_| {
        assert!(inline_for::<T>(&Typescript::default()).is_err());
        assert!(
            inline_for::<T>(&Typescript::default().bigint(BigIntExportBehavior::Fail)).is_err()
        );

        assert_eq!(
            inline_for::<T>(&Typescript::default().bigint(BigIntExportBehavior::String)).unwrap(),
            "string"
        );
        assert_eq!(
            inline_for::<T>(&Typescript::default().bigint(BigIntExportBehavior::Number)).unwrap(),
            "number"
        );
        assert_eq!(
            inline_for::<T>(&Typescript::default().bigint(BigIntExportBehavior::BigInt)).unwrap(),
            "bigint"
        );
    });
}

#[test]
fn bigint_errors_propagate_from_nested_types() {
    let ts = Typescript::default();

    for err in [
        inline_for::<StructWithBigInt>(&ts),
        inline_for::<StructWithStructWithBigInt>(&ts),
        inline_for::<StructWithStructWithStructWithBigInt>(&ts),
        inline_for::<StructWithOptionWithStructWithBigInt>(&ts),
        inline_for::<EnumWithInlineStructWithBigInt>(&ts),
    ] {
        let err = err.expect_err("bigint export should be rejected by default");
        assert!(
            err.to_string().contains("forbids exporting BigInt types"),
            "unexpected error: {err}"
        );
    }

    assert!(inline_for::<EnumWithStructWithStructWithBigInt>(&ts).is_ok());
}
