use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type)]
pub struct Testing<T = String>(T);

fn main() {
    println!(
        "{}",
        Typescript::default()
            .export(&specta_serde::apply(Types::default().register::<Testing>()).unwrap(),)
            .unwrap()
    );
}
