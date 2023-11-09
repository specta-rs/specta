use std::marker::PhantomData;

use specta_rpc::router;

router!(Router::<H, M> where H: Handler<M>);

pub trait Handler<M> {}

pub struct Marker<T, R>(PhantomData<(T, R)>);
impl<T, R, F: Fn((), T) -> R> Handler<Marker<T, R>> for F {}

fn main() {
    let router = Router::new().mount("demo", |ctx, arg: ()| "Hello, World!");
    // .mount("demo", |ctx, arg: ()| async move { "Hello, World!" });
}
