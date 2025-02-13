use specta::Type;
use specta_typescript::Typescript;

#[derive(Type)]
pub struct TypeOne {
    pub field1: String,
    pub field2: TypeTwo,
}

#[derive(Type)]
pub struct TypeTwo {
    pub my_field: String,
}

fn main() {
    Typescript::default()
        // This requires the `export` feature to be enabled on Specta
        .export_to("./bindings.ts", &specta::export())
        .unwrap();

    let result = std::fs::read_to_string("./bindings.ts").unwrap();
    println!("{result}");
    assert_eq!(result, r#"todo"#);
}
