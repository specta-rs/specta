use specta::Type;
use specta_typescript::{JSDoc, Typescript};

/// Hello World
#[derive(Type)]
pub struct TypeOne {
    pub field1: String,
    pub field2: TypeTwo,
}

/// Bruh
#[derive(Type)]
#[deprecated = "bruh"]
pub struct TypeTwo {
    #[deprecated]
    pub my_field: String,
    /// Another one
    pub bruh: another::TypeThree,
}

#[derive(Type)]
pub struct ImGeneric<T> {
    pub my_field: T,
}

mod another {
    #[derive(specta::Type)]
    pub struct TypeThree {
        pub my_field: String,
    }
}

fn main() {
    Typescript::default()
        .format(specta_typescript::Format::Files)
        // This requires the `export` feature to be enabled on Specta
        .export_to("./bindings", &specta::export())
        .unwrap();

    JSDoc::default()
        .format(specta_typescript::Format::Files)
        // This requires the `export` feature to be enabled on Specta
        .export_to("./bindings2", &specta::export())
        .unwrap();
}
