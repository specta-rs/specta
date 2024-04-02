use specta::{specta, Type};

pub struct A;

// #[specta]
// #[specta_macros::testing]
// fn hello_world(a: String) -> String {
//     todo!();
// }

// fn hello_world2(a: B<String, _>) -> String {
//     todo!();
// }

// #[specta(unstable_map_params = B<T, _>)]
// fn hello_world() -> String {
//     todo!();
// }

// TODO: Ignore `A`
// #[specta]
// fn hello_world2(a: A) {}

fn main() {
    // println!("{:?}", testing());
    test2(); // TODO
}

fn test<A>()
where
    A: std::fmt::Debug,
{
}

fn test2() {}

pub struct B<T, K>(T, K);

impl<T: Type> Type for B<T, ()> {
    fn inline(type_map: &mut specta::TypeMap, generics: specta::Generics) -> specta::DataType {
        todo!()
    }
}

impl<R: tauri::Runtime> Type for B<tauri::Window<R>, ((),)> {
    fn inline(type_map: &mut specta::TypeMap, generics: specta::Generics) -> specta::DataType {
        todo!()
    }
}
