use specta::{Type, TypeCollection};

#[derive(Type)]
pub struct A {
    bigint: u128,
    date: chrono::DateTime<chrono::Utc>,
    bytes: bytes::Bytes,
    b: B,
}

#[derive(Type)]
pub struct B {
    bigint: u128,
    date: chrono::DateTime<chrono::Utc>,
    bytes: bytes::Bytes,
}

fn main() {
    let mut types = TypeCollection::default();
    let dt = A::definition(&mut types);

    let tags = specta_tags::v2::TransformPlan::analyze(dt, &types);
    println!("--- PLAN ---\n{tags:?}");
    // This would be emitted for each Tauri Specta command.
    println!("--- RESULT ---\n result.then((v) => {})", tags.map("v"));
}
