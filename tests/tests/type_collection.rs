use specta::{Type, TypeCollection};
use specta_util::TypeCollection;

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
    let mut type_map = TypeCollection::default();
    TypeCollection::default()
        .register::<A>()
        .collect(&mut type_map);
    assert_eq!(type_map.len(), 2);
}

#[test]
fn type_collection_merge() {
    let mut a = TypeCollection::default();
    a.register::<A>();
    let mut b = TypeCollection::default();
    b.register::<C>();

    let mut type_map = TypeCollection::default();
    TypeCollection::default()
        .register::<D>()
        .extend(a)
        .extend(b)
        .collect(&mut type_map);
    assert_eq!(type_map.len(), 4);

    // Check it compile with any valid arg
    TypeCollection::default()
        .extend(&TypeCollection::default())
        .extend(&mut TypeCollection::default())
        .extend(TypeCollection::default());
}

#[test]
fn type_collection_duplicate_register_ty() {
    let mut type_map = TypeCollection::default();
    TypeCollection::default()
        .register::<C>()
        .register::<C>()
        .collect(&mut type_map);
    assert_eq!(type_map.len(), 1);
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
