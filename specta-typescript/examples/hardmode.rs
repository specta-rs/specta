use specta::{
    datatype::{Generic, GenericPlaceholder},
    Type, TypeCollection,
};
use specta_typescript::Typescript;

pub trait Demo {
    type T<A>;
}
impl Demo for () {
    type T<A> = Option<A>;
}

pub trait Demo2: Sized {
    type T: Type;
}
// If we `+ SomeTrait` `Entity2` will break unless we `impl SomeTrait for GenericPlaceholder` somehow.
impl<T: Type> Demo2 for T {
    type T = Option<T>;
}

#[derive(Type)]
pub struct Entity1<A> {
    pub a: <() as Demo>::T<A>,
}

#[derive(Type)]
pub struct Entity2<B: Demo2> {
    pub a: <B as Demo2>::T,
}

// TODO

#[derive(Type)]
pub struct Entity<A, B: Demo2> {
    pub a: <() as Demo>::T<A>,
    pub b: <B as Demo2>::T,
}

// Entity<GenericA, GenericB>

#[derive(Type)]
pub struct SomeType {
    pub a: Entity<i32, i32>,
    pub b: Entity<bool, bool>,
}

fn main() {
    // TODO: Handling multiple generics
    pub struct GenericA;
    impl GenericPlaceholder for GenericA {
        const PLACEHOLDER: &'static str = "A";
    }
    let todo = <Option<Generic<GenericA>> as Type>::definition(&mut TypeCollection::default());
    println!("{:?}", todo);

    let types = TypeCollection::default()
        // These will register the same as a registration maintains the generic.
        .register::<Entity<(), ()>>()
        .register::<Entity<i32, i32>>()
        .register::<SomeType>();

    Typescript::default()
        .export_to("./bindings.ts", &types)
        .unwrap();

    let result = std::fs::read_to_string("./bindings.ts").unwrap();
    println!("{result}");
    // TODO: Assertion
}
