use specta::Type;
use specta_typescript::{BigIntExportBehavior, Typescript};

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
    // Export as string
    let string = Typescript::default().export(&specta::export()).unwrap();
    println!("{string}");

    // Export to file
    Typescript::default()
        .export_to("./bindings.ts", &specta::export())
        .unwrap();

    // Override the export configuration.
    Typescript::default()
        // Be aware this won't be typesafe unless your using a ser/deserializer that converts BigInt types to a number.
        .bigint(BigIntExportBehavior::Number)
        .export_to("./bindings.ts", &specta::export())
        .unwrap();
}
