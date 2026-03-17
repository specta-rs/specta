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

    let tags = specta_tags::v2::Tags::analyze(dt);
    println!("PLAN: {tags:?}");
}
