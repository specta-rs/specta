use specta::ts::{BigIntExportBehavior, ExportConfiguration};

macro_rules! for_bigint_types {
    (T -> $s:expr) => {{
        for_bigint_types!(usize, isize, i64, u64, i128, u128; $s);
    }};
    ($($i:ty),+; $s:expr) => {{
        $({
            type T = $i;
            $s
        })*
    }};
}

#[test]
fn test_bigint_types() {
    // TODO: Assert error type is exactly what is expected for these ones
    for_bigint_types!(T -> assert!(specta::ts::inline::<T>(&ExportConfiguration::default()).is_err()));
    for_bigint_types!(T -> assert!(specta::ts::inline::<T>(&ExportConfiguration::new()).is_err()));
    for_bigint_types!(T -> assert!(specta::ts::inline::<T>(&ExportConfiguration::new().bigint(BigIntExportBehavior::Fail)).is_err()));
    for_bigint_types!(T -> assert!(specta::ts::inline::<T>(&ExportConfiguration::new().bigint(BigIntExportBehavior::FailWithReason("some reason"))).is_err()));

    for_bigint_types!(T -> assert_eq!(specta::ts::inline::<T>(&ExportConfiguration::new().bigint(BigIntExportBehavior::String)), Ok("string".into())));
    for_bigint_types!(T -> assert_eq!(specta::ts::inline::<T>(&ExportConfiguration::new().bigint(BigIntExportBehavior::Number)), Ok("number".into())));
    for_bigint_types!(T -> assert_eq!(specta::ts::inline::<T>(&ExportConfiguration::new().bigint(BigIntExportBehavior::BigInt)), Ok("BigInt".into())));
}
