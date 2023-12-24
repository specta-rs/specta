use specta::Type;

#[derive(Type)]
pub struct Demo {
    pub a: String,
    pub b: i32,
    pub c: bool,
    // TODO: Reference other structs still
}
