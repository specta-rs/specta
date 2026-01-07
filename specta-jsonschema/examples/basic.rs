use specta::{Type, TypeCollection};
use specta_jsonschema::{JsonSchema, SchemaVersion};

#[derive(Type)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: Option<String>,
    pub role: Role,
}

#[derive(Type)]
pub enum Role {
    Admin,
    User,
    Guest,
}

#[derive(Type)]
pub struct Post {
    pub id: u32,
    pub title: String,
    pub content: String,
    pub author_id: u32,
    pub tags: Vec<String>,
}

fn main() {
    // Create a type collection with all the types we want to export
    let types = TypeCollection::default()
        .register::<User>()
        .register::<Role>()
        .register::<Post>();

    // Export to JSON Schema (Draft 7)
    let schema = JsonSchema::default()
        .schema_version(SchemaVersion::Draft7)
        .title("My API Types")
        .description("JSON Schema for my API types")
        .export(&types)
        .unwrap();

    println!("{}", schema);
}
