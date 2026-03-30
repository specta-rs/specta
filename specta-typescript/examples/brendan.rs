use specta::{ResolvedTypes, Type, Types};
use specta_typescript::Typescript;

#[derive(Clone, serde::Serialize, Type)]
#[serde(tag = "phase", rename_all = "snake_case")]
enum Testing {
    A,
    B,
    C,
}

fn main() {
    let result = Typescript::default()
        .export(&ResolvedTypes::from_resolved_types(
            Types::default().register::<Testing>(),
        ))
        .unwrap();
    println!("{}", result);
}
