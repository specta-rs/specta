use std::marker::PhantomData;

use specta_rpc::router;

pub trait Handler<Ctx, M> {}

pub struct Marker<T, R>(PhantomData<(T, R)>);
impl<Ctx, T, R, F: Fn(Ctx, T) -> R> Handler<Ctx, Marker<T, R>> for F {}

router!(Router::<Ctx> [H, M] where H: Handler<Ctx, M>);

fn main() {
    let router = Router::<String>::new()
        .mount("demo", |ctx, arg: ()| "Hello, World Sync!")
        .mount("demo", |ctx, arg: ()| async move { "Hello, World Async!" });
}
