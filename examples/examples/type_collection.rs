use specta::{Type, TypeCollection};
use specta_typescript::Typescript;

#[derive(Type)]
pub struct Hello {
    pub a: i32,
    pub b: bool,
}

#[derive(Type)]
pub struct Test(Hello);

fn main() {
    let code = Typescript::default()
        // You can use `export_to` to export to a file
        .export(
            TypeCollection::default()
                .register::<Hello>()
                .register::<Test>(),
        )
        .unwrap();

    assert_eq!(
        code,
        "export type Hello = { a: number; b: boolean }\nexport type Test = Hello\n"
    );
}
