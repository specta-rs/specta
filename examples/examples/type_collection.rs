use specta::Type;
use specta_typescript::Typescript;
use specta_util::TypeCollection;

#[derive(Type)]
pub struct Hello {
    pub a: i32,
    pub b: bool,
}

#[derive(Type)]
pub struct Test(Hello);

fn main() {
    let code = TypeCollection::default()
        .register::<Hello>()
        .register::<Test>()
        .export(&Typescript::default())
        .unwrap();

    assert_eq!(
        code,
        "export type Hello = { a: number; b: boolean }\nexport type Test = Hello\n"
    )
}
