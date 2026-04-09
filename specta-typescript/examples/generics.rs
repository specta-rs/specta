// use specta::{Type, Types};
// use specta_typescript::Typescript;

// // TODO: We need to error better on this condition, we should forbid associated constants (but can we even tell which are it???)

// trait Bruh {
//     const LEN: usize;
// }
// impl Bruh for usize {
//     const LEN: usize = 4;
// }
// impl Bruh for u8 {
//     const LEN: usize = 2;
// }

// #[derive(Type)]
// #[specta(collect = false)]
// struct Demo<T: Bruh> {
//     data: [u32; T::LEN],
// }

// #[derive(Type)]
// struct ContainsDemo {
//     a: Demo<u8>,
//     b: Demo<usize>,
// }

fn main() {
    //     println!(
    //         "{}",
    //         Typescript::default()
    //             .export(&specta_serde::apply(Types::default().register::<ContainsDemo>()).unwrap(),)
    //             .unwrap()
    //     );
}
