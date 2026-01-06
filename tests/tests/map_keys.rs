use std::{collections::HashMap, convert::Infallible};

use specta::{Type, TypeCollection};
use specta_serde::Error;
use specta_typescript::Any;

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
    insta::assert_snapshot!(crate::ts::inline::<HashMap<String, ()>>(&Default::default()).unwrap(), @"{ [key in string]: null }");
    insta::assert_snapshot!(crate::ts::export::<Regular>(&Default::default()).unwrap(), @"export type Regular = { [key in string]: null };");
    insta::assert_snapshot!(crate::ts::inline::<HashMap<Infallible, ()>>(&Default::default()).unwrap(), @"{ [key in never]: null }");
    insta::assert_snapshot!(crate::ts::inline::<HashMap<Any, ()>>(&Default::default()).unwrap(), @"Partial<{ [key in any]: null }>");
    insta::assert_snapshot!(crate::ts::inline::<HashMap<TransparentStruct, ()>>(&Default::default()).unwrap(), @"{ [key in string]: null }");
    insta::assert_snapshot!(crate::ts::inline::<HashMap<UnitVariants, ()>>(&Default::default()).unwrap(), @"Partial<{ [key in \"A\" | \"B\" | \"C\"]: null }>");
    insta::assert_snapshot!(crate::ts::inline::<HashMap<UntaggedVariants, ()>>(&Default::default()).unwrap(), @"Partial<{ [key in string | number]: null }>");
    insta::assert_snapshot!(crate::ts::inline::<ValidMaybeValidKey>(&Default::default()).unwrap(), @"{ [key in string]: null }");
    insta::assert_snapshot!(crate::ts::export::<ValidMaybeValidKey>(&Default::default()).unwrap(), @"export type ValidMaybeValidKey = { [key in MaybeValidKey<string>]: null };");

    insta::assert_snapshot!(crate::ts::inline::<ValidMaybeValidKeyNested>(&Default::default()).unwrap(), @"{ [key in string]: null }");
    insta::assert_snapshot!(crate::ts::export::<ValidMaybeValidKeyNested>(&Default::default()).unwrap(), @"export type ValidMaybeValidKeyNested = { [key in MaybeValidKey<MaybeValidKey<string>>]: null };");

    insta::assert_snapshot!(check::<HashMap<() /* `null` */, ()>>().unwrap_err(), @"InvalidMapKey");
    insta::assert_snapshot!(check::<HashMap<RegularStruct, ()>>().unwrap_err(), @"InvalidMapKey");
    insta::assert_snapshot!(check::<HashMap<Variants, ()>>().unwrap_err(), @"InvalidMapKey");
    insta::assert_snapshot!(check::<InvalidMaybeValidKey>().unwrap_err(), @"InvalidMapKey");
    insta::assert_snapshot!(check::<InvalidMaybeValidKeyNested>().unwrap_err(), @"InvalidMapKey");
    // TODO: detected a recursive inline
}

fn check<T: Type>() -> Result<(), String> {
    let mut types = TypeCollection::default();
    let dt = T::definition(&mut types);
    specta_serde::validate(&types).map_err(|e| format!("{:?}", e))
}
