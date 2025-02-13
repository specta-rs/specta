use specta::{Type, TypeCollection};

#[derive(Type)]
struct A2(String);

#[derive(Type)]
struct A {
    a: A2,
}

#[derive(Type)]
struct C {
    d: String,
}

#[derive(Type)]
struct D(String);

#[test]
fn type_collection_export() {
    let types = TypeCollection::default().register::<A>();
    assert_eq!(types.len(), 2);
}

#[test]
fn type_collection_merge() {
    let types = TypeCollection::default()
        .register::<D>()
        .extend(TypeCollection::default().register::<A>())
        .extend(TypeCollection::default().register::<C>());
    assert_eq!(types.len(), 4);

    // Check it compile with any valid arg
    TypeCollection::default()
        .extend(&TypeCollection::default())
        .extend(&mut TypeCollection::default())
        .extend(TypeCollection::default());
}

#[test]
fn type_collection_duplicate_register_ty() {
    let types = TypeCollection::default().register::<C>().register::<C>();

    assert_eq!(types.len(), 1);
}

// TODO: Bring this back
// #[test]
// #[cfg(feature = "typescript")]
// fn type_collection_ts() {
//     let result = TypeCollection::default()
//         .register::<A>()
//         .register::<C>()
//         .register::<D>()
//         .export_ts(&Default::default())
//         .unwrap();
//     assert_eq!(
//         result,
//         "export type A = { a: A2 }\nexport type A2 = string\nexport type C = { d: string }\nexport type D = string\n"
//     );
// }
