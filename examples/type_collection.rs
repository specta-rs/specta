use specta::{ts::ExportConfig, Type, TypeCollection};

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
        .export_ts(&ExportConfig::default())
        .unwrap();

    assert_eq!(
        code,
        "export type Hello = { a: number; b: boolean }\nexport type Test = Hello\n"
    )
}
