use specta::{Type, Types};
use specta_rescript::ReScript;

/// A parent in a mutually recursive relationship.
#[derive(Type)]
struct Parent {
    child: Option<Box<Child>>,
}

/// A child in a mutually recursive relationship.
#[derive(Type)]
struct Child {
    parent: Option<Box<Parent>>,
}

pub fn types() -> Types {
    Types::default().register::<Parent>()
}

fn main() {
    ReScript::default()
        .without_serde()
        .export_to(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/examples/generated/Recursive.res"
            ),
            &types(),
        )
        .unwrap();
}
