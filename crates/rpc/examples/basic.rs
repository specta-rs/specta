use std::{fmt::Debug, future::Future, marker::PhantomData};

use specta::Type;
use specta_rpc::router;

pub trait Handler<Ctx, M> {}

pub struct SyncMarker<T, R>(PhantomData<(T, R)>);
impl<TContext, TArgument, R: Debug + Type, F: Fn(TContext, TArgument) -> R>
    Handler<TContext, SyncMarker<TArgument, R>> for F
{
}

pub struct AsyncMarker<TFunction>(PhantomData<TFunction>);
impl<TContext, TArgument, TFunction, TFuture> Handler<TContext, AsyncMarker<TArgument>>
    for TFunction
where
    TFunction: Fn(TContext, TArgument) -> TFuture,
    TFuture: Future,
    TFuture::Output: Debug,
{
}

router!(Router::<Ctx> [H, M] where H: Handler<Ctx, M>);

fn main() {
    let router = Router::<String>::new()
        .mount("demo", |ctx, arg: ()| "Hello, World Sync!")
        .mount("demo", |ctx, arg: ()| async move { "Hello, World Async!" });

    // TODO: Export types

    // TODO: Runtime demo
}
