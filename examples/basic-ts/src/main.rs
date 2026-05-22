use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

// Specta works by deriving `Type` for the Rust types you want to export.
//
// `Serialize` and `Deserialize` are not required by Specta itself, but they are
// common in real applications and allow Specta to understand serde attributes
// when exporting through `specta_serde::Format` below.
#[derive(Serialize, Deserialize, Type)]
pub struct User {
    id: u32,
    name: String,

    // Serde attributes are respected when using `specta_serde::Format`.
    // This field will be exported to TypeScript as `emailAddress`.
    #[serde(rename = "emailAddress")]
    email_address: String,

    // Optional Rust values become `T | null` in TypeScript.
    avatar_url: Option<String>,
}

#[derive(Serialize, Deserialize, Type)]
pub struct Post {
    id: u32,
    title: String,
    author: User,
    tags: Vec<String>,
    status: PostStatus,
}

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PostStatus {
    Draft,
    InReview,
    Published,
}

fn main() {
    // Register every root type you want to export. Specta automatically follows
    // references, so registering `Post` is enough to include `User` and
    // `PostStatus`, but registering roots explicitly is often clearer in apps.
    let types = Types::default()
        .register::<User>()
        .register::<Post>()
        .register::<PostStatus>();

    // Export the registered Rust types as TypeScript definitions.
    //
    // `specta_serde::Format` tells Specta to apply serde's wire-format rules,
    // such as `rename`, `rename_all`, `tag`, `untagged`, and `flatten`.
    //
    // You can also use `specta_serde::FormatPhases` to allow types to be
    // narrowed based on if your serializing or deserializing.
    let output = Typescript::default()
        .export(&types, specta_serde::Format)
        .expect("failed to export TypeScript types");

    println!("{output}");
}
