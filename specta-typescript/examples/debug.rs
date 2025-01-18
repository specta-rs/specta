use specta::{datatype, Type};

// #[derive(Type)]
// #[specta(export = false)]
// struct Generic<T> {
//     t: T,
// }

// #[derive(Type)]
// #[specta(export = false)]
// struct Container {
//     g: Generic<String>,
//     #[specta(inline)]
//     gi: Generic<String>,
//     #[specta(flatten)]
//     t: Generic<String>,
// }

// #[derive(Type)]
// #[specta(export = false)]
// struct Bar {
//     field: i32,
// }

// #[derive(Type)]
// #[specta(export = false)]
// struct Foo {
//     bar: Bar,
// }

#[derive(Type)]
#[specta(export = false)]
struct B {
    b: u32,
}

#[derive(Type)]
#[specta(export = false)]
struct A {
    a: B,
    #[specta(inline)]
    b: B,
    // #[specta(flatten)]
    // c: B,
    // #[specta(inline, flatten)]
    // d: D,
    // #[specta(inline, flatten)]
    // e: GenericFlattened<u32>,
}

fn main() {
    // println!("{:?}\n\n", <Container as Type>::definition(&mut Default::default()));

    // println!("{:#?}", Foo::definition(&mut Default::default()));
    // println!("{:#?}", datatype::inline::<Foo>(&mut Default::default(), &[]));

    // println!("{:?}", specta_typescript::inline::<Foo>(&Default::default()));
    // println!("{:?}", specta_typescript::export::<Foo>(&Default::default()));
    // export type Container = ({ t: string }) & { g: Generic<string>; gi: { t: string } }
}
