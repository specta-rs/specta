//! This file is run with the `trybuild` crate to assert associated item errors.

use specta::Type;

trait MyTrait {
    type Assoc: Type;
    const LEN: usize;
}

#[derive(Type)]
#[specta(collect = false)]
struct DirectAssociatedType<T: MyTrait> {
    value: T::Assoc,
}

#[derive(Type)]
#[specta(collect = false)]
struct QualifiedAssociatedType<T: MyTrait> {
    value: <T as MyTrait>::Assoc,
}

#[derive(Type)]
#[specta(collect = false)]
struct AssociatedConst<T: MyTrait> {
    #[specta(type = [u8; T::LEN])]
    value: std::marker::PhantomData<T>,
}

#[derive(Type)]
#[specta(collect = false, type = T::Assoc)]
struct AssociatedTypeOverride<T: MyTrait> {
    value: std::marker::PhantomData<T>,
}

fn main() {}
