use specta::Type;

#[derive(Type)]
pub struct A {
    data: String,
}

#[derive(Type)]
pub struct B {
    a: A,
    b: nested::B,
}

mod nested {
    use specta::Type;

    #[derive(Type)]
    pub struct B {
        age: u16,
    }
}

mod nested2 {
    use specta::Type;

    // What if the user replaces a std type with their own
    #[derive(Type)]
    pub struct String(i32);

    #[derive(Type)]
    pub struct B {
        s: String,
    }
}

mod nested3 {
    use specta::Type;

    use super::nested2::String;

    #[derive(Type)]
    pub struct C {
        s: String,
    }
}
