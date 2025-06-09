use std::{collections::HashMap, convert::Infallible};

use specta::{Type, TypeCollection};
use specta_serde::Error;
use specta_typescript::Any;

use crate::ts::{assert_ts, assert_ts_export};

// Export needs a `NamedDataType` but uses `Type::reference` instead of `Type::inline` so we test it.
#[derive(Type)]
#[specta(collect = false)]
struct Regular(HashMap<String, ()>);

#[derive(Type)]
#[specta(collect = false)]
struct RegularStruct {
    a: String,
}

#[derive(Type)]
#[specta(collect = false, transparent)]
struct TransparentStruct(String);

#[derive(Type)]
#[specta(collect = false)]
enum UnitVariants {
    A,
    B,
    C,
}

#[derive(Type)]
#[specta(collect = false, untagged)]
enum UntaggedVariants {
    A(String),
    B(i32),
    C(u8),
}

#[derive(Type)]
#[specta(collect = false, untagged)]
enum InvalidUntaggedVariants {
    A(String),
    B(i32, String),
    C(u8),
}

#[derive(Type)]
#[specta(collect = false)]
enum Variants {
    A(String),
    B(i32),
    C(u8),
}

#[derive(Type)]
#[specta(collect = false, transparent)]
pub struct MaybeValidKey<T>(T);

#[derive(Type)]
#[specta(collect = false, transparent)]
pub struct ValidMaybeValidKey(HashMap<MaybeValidKey<String>, ()>);

#[derive(Type)]
#[specta(collect = false, transparent)]
pub struct ValidMaybeValidKeyNested(HashMap<MaybeValidKey<MaybeValidKey<String>>, ()>);

#[derive(Type)]
#[specta(collect = false, transparent)]
pub struct InvalidMaybeValidKey(HashMap<MaybeValidKey<()>, ()>);

#[derive(Type)]
#[specta(collect = false, transparent)]
pub struct InvalidMaybeValidKeyNested(HashMap<MaybeValidKey<MaybeValidKey<()>>, ()>);

#[test]
fn map_keys() {
    assert_ts!(HashMap<String, ()>, "{ [key in string]: null }");
    assert_ts_export!(Regular, "export type Regular = { [key in string]: null };");
    assert_ts!(HashMap<Infallible, ()>, "{ [key in never]: null }");
    assert_ts!(HashMap<Any, ()>, "Partial<{ [key in any]: null }>");
    assert_ts!(HashMap<TransparentStruct, ()>, "{ [key in string]: null }");
    assert_ts!(HashMap<UnitVariants, ()>, "Partial<{ [key in \"A\" | \"B\" | \"C\"]: null }>");
    assert_ts!(HashMap<UntaggedVariants, ()>, "Partial<{ [key in string | number]: null }>");
    assert_ts!(ValidMaybeValidKey, "{ [key in string]: null }");
    assert_ts_export!(
        ValidMaybeValidKey,
        "export type ValidMaybeValidKey = { [key in MaybeValidKey<string>]: null };"
    );

    assert_ts!(ValidMaybeValidKeyNested, "{ [key in string]: null }");
    assert_ts_export!(
        ValidMaybeValidKeyNested,
        "export type ValidMaybeValidKeyNested = { [key in MaybeValidKey<MaybeValidKey<string>>]: null };"
    );

    assert_eq!(
        check::<HashMap<() /* `null` */, ()>>(),
        Err(Error::InvalidMapKey)
    );
    assert_eq!(
        check::<HashMap<RegularStruct, ()>>(),
        Err(Error::InvalidMapKey)
    );
    assert_eq!(check::<HashMap<Variants, ()>>(), Err(Error::InvalidMapKey));
    assert_eq!(check::<InvalidMaybeValidKey>(), Err(Error::InvalidMapKey));
    assert_eq!(
        check::<InvalidMaybeValidKeyNested>(),
        Err(Error::InvalidMapKey)
    ); // TODO: detected a recursive inline
}

fn check<T: Type>() -> Result<(), Error> {
    let mut types = TypeCollection::default();
    let dt = T::definition(&mut types);
    specta_serde::validate_dt(&dt, &types)
}
