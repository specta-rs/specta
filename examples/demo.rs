use specta::{datatype::TypeImpl, Type};

#[derive(Type)]
pub struct Demo {
    a: String,
}

fn main() {
    // let a = TypeImpl::new::<String>();
    // let b = TypeImpl::new::<i32>();
    // let c = TypeImpl::new::<Demo>();
    // println!("{a:?}\n{b:?}\n{c:?}");
}
