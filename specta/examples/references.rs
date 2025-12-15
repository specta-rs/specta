use std::{ptr, sync::Arc};

fn main() {
    let t = Arc::new(());
    let t2 = t.clone();
    let t3 = Arc::new(());

    println!("{:?}", Arc::ptr_eq(&t, &t2));
    println!("{:?}", Arc::ptr_eq(&t, &t3));
    println!("{:?}", t);
    println!("{:?}", t2);
    println!("{:?}", t3);

    // let todo = Box::new(());
    // println!("{:?} {:?}", &raw const todo, &raw const todo as u64);

    // drop(todo);

    // let todo = Box::new(());
    // println!("{:?} {:?}", size_of_val(&todo), size_of::<()>());
    // println!("{:?} {:?}", &raw const todo, &raw const todo as u64);

    // TODO: What if the heap is dropped?
}
