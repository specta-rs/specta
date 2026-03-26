//! Basic `specta-tags` usage.

use specta::{ResolvedTypes, Type, Types};

#[allow(dead_code)]
#[derive(Type)]
struct A {
    bigint: u128,
    date: chrono::DateTime<chrono::Utc>,
    bytes: bytes::Bytes,
    b: B,
}

#[allow(dead_code)]
#[derive(Type)]
struct B {
    bigint: u128,
    date: chrono::DateTime<chrono::Utc>,
    bytes: bytes::Bytes,
}

fn main() {
    let mut types = Types::default();
    let dt = A::definition(&mut types);
    let resolved = ResolvedTypes::from_resolved_types(types);

    let tags = specta_tags::TransformPlan::analyze(dt, &resolved);
    println!("--- PLAN ---\n{tags:?}");
    println!("--- RESULT ---\nresult.then((v) => {})", tags.map("v"));
}
