use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type)]
struct Demo<const N: usize = 1> {
    data: [u32; N], // becomes `number[]`
    a: [u8; 2],     // becomes `number[]`
    #[specta(type = specta_util::FixedArray<2, u8>)]
    d: [u8; 2], // becomes `[number number]`
}

#[derive(Type)]
struct ContainsDemo {
    a: Demo,    // becomes `Demo`
    b: Demo<2>, // becomes `Demo`
    d: [u8; 2], // becomes `[number, number]`
}

fn main() {
    println!(
        "{}",
        Typescript::default()
            .export(&specta_serde::apply(Types::default().register::<ContainsDemo>()).unwrap(),)
            .unwrap()
    );
}
