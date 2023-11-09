use std::{any::Any, collections::HashMap};

use specta::{
    ts::{self, ExportConfig},
    DefOpts, Type, TypeMap,
};

// #[derive(Type)]
// pub struct TypeOne {
//     pub field1: String,
//     pub field2: i32,

//     // Overriding the field type doesn't effect serde so your JSON and types may not match but if you know what your doing this is useful
//     #[specta(type = String)]
//     pub override_type: i32,
// }

// #[derive(Type)]
// pub struct GenericType<A> {
//     pub my_field: String,
//     pub generic: A,
// }

// #[derive(Type, Hash)]
// pub enum MyEnum {
//     A,
//     B,
//     C,
// }

// #[derive(Type)]
// pub struct Something {
//     a: HashMap<MyEnum, i32>,
// }

// #[derive(Type)]
// #[serde(transparent)]
// pub struct Something2(HashMap<String, Something3>);

// #[derive(Type)]
// pub struct Something3(String, i32);

// #[derive(Type)]
// pub struct Demo2 {
//     b: String,
//     a: Option<Something>,
// }

// #[derive(Type)]
// pub enum Demo {
//     #[serde(skip)]
//     A(Box<dyn Any>),
// }

#[derive(Type)]
#[specta(transparent)]
pub struct MaybeValidKey<U, T>(T, #[specta(skip)] U);

#[derive(Type)]
#[specta(transparent)]
pub struct ValidMaybeValidKey(HashMap<MaybeValidKey<(), MaybeValidKey<(), String>>, ()>);

fn main() {
    // let ts_str = ts::export::<Something2>(&ExportConfig::default()).unwrap();
    // println!("{ts_str}");

    // let c = ExportConfig::default();
    // let ts_str = ts::export::<Demo>(&c).unwrap();
    // println!("{ts_str}");

    // let mut type_map = TypeMap::default();
    // let ty = ValidMaybeValidKey::inline(
    //     DefOpts {
    //         parent_inline: false,
    //         type_map: &mut type_map,
    //     },
    //     &[],
    // );
    // println!("{:#?}", type_map)

    let ts_str = ts::export::<ValidMaybeValidKey>(&ExportConfig::default()).unwrap();
    println!("{ts_str}");
}
