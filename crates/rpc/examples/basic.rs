use std::{fmt::Debug, future::Future, marker::PhantomData};

use specta::Type;
use specta_rpc::{router, IntoHandler};

pub trait HandlerFn<TContext, M> {
    fn build(self) -> DynHandlerFn<TContext>;
}

// pub enum IntoHandler {}

// impl<TContext, M, F: HandlerFn<TContext, M>> FromRouter<F> for Router<TContext> {}

pub struct SyncMarker<T, R>(PhantomData<(T, R)>);
impl<TContext, TArgument, R: Debug + Type, F: Fn(TContext, TArgument) -> R>
    HandlerFn<TContext, SyncMarker<TArgument, R>> for F
{
    fn build(self) -> DynHandlerFn<TContext> {
        todo!()
    }
}

pub struct AsyncMarker<TFunction>(PhantomData<TFunction>);
impl<TContext, TArgument, TFunction, TFuture> HandlerFn<TContext, AsyncMarker<TArgument>>
    for TFunction
where
    TFunction: Fn(TContext, TArgument) -> TFuture,
    TFuture: Future,
    TFuture::Output: Debug,
{
    fn build(self) -> DynHandlerFn<TContext> {
        todo!()
    }
}

router!(Router::<Ctx> [H, M] where H: HandlerFn<Ctx, M>);

type DynHandlerFn<TContext> =
    Box<dyn Fn(TContext, serde_json::Value) -> Box<dyn Future<Output = serde_json::Value>>>;

impl<TContext> IntoHandler for Router<TContext> {
    type Handler = DynHandlerFn<TContext>;
}

fn main() {
    let router = Router::<String>::new()
        .mount("demo", |ctx, arg: ()| "Hello, World Sync!")
        .mount("demo", |ctx, arg: ()| async move { "Hello, World Async!" });

    // TODO: Export types

    // TODO: Runtime demo
}
