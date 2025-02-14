use specta::{Type, TypeCollection};

#[derive(Type)]
#[specta(untagged)]
pub enum GenericType<T> {
    Undefined,
    Value(T),
}

fn main() {
    let mut types = TypeCollection::default();
    println!(
        "{:?}\n{:#?}\n{:?}\n{:?}",
        GenericType::<i32>::definition(&mut types),
        types,
        specta_typescript::legacy::export::<GenericType::<i32>>(&Default::default()),
        specta_typescript::legacy::inline::<GenericType::<i32>>(&Default::default())
    );
}
