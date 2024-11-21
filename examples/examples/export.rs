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
    let string = specta_util::export().export(Typescript::default()).unwrap();
    println!("{string}");

    // Export to file
    specta_util::export()
        .export_to(Typescript::default(), "./bindings.ts")
        .unwrap();

    // Override the export configuration.
    specta_util::export()
        .export_to(
            Typescript::default()
                // Be aware this won't be typesafe unless your using a ser/deserializer that converts BigInt types to a number.
                .bigint(BigIntExportBehavior::Number),
            "./bindings.ts",
        )
        .unwrap();
}
