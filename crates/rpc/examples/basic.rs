//! This example shows an rspc-like router implemented using specta-rpc.
//!
//! Your functions can be either sync or async and take two arguments.
//! The first argument is a fixed context type and the second is a generic per-action argument.

use std::{
    future::{ready, Future},
    marker::PhantomData,
    pin::Pin,
};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::json;
use specta::Type;
use specta_rpc::{router, IntoHandler};

pub trait HandlerFn<TContext, M> {
    fn build(self) -> DynHandlerFn<TContext>;
}

pub struct SyncMarker<T, R>(PhantomData<(T, R)>);
impl<TContext, TArgument, TResult, TFunction> HandlerFn<TContext, SyncMarker<TArgument, TResult>>
    for TFunction
where
    TFunction: Fn(TContext, TArgument) -> TResult + 'static,
    TArgument: DeserializeOwned + Type,
    TResult: Serialize + Type,
{
    fn build(self) -> DynHandlerFn<TContext> {
        Box::new(move |ctx, input| {
            let result =
                // TODO: Error handling
                serde_json::to_value((self)(ctx, serde_json::from_value(input).unwrap())).unwrap();
            Box::pin(ready(result))
        })
    }
}

pub struct AsyncMarker<TFunction>(PhantomData<TFunction>);
impl<TContext, TArgument, TFunction, TFuture> HandlerFn<TContext, AsyncMarker<TArgument>>
    for TFunction
where
    TFunction: Fn(TContext, TArgument) -> TFuture + 'static,
    TFuture: Future + 'static,
    TArgument: DeserializeOwned + Type,
    TFuture::Output: Serialize + Type,
{
    fn build(self) -> DynHandlerFn<TContext> {
        Box::new(move |ctx, input| {
            // TODO: Error handling
            let result = (self)(ctx, serde_json::from_value(input).unwrap());
            Box::pin(async move {
                // TODO: Error handling
                serde_json::to_value(result.await).unwrap()
            })
        })
    }
}

router!(Router::<Ctx> [H => |h| h.build(), M] where H: HandlerFn<Ctx, M>);

type DynHandlerFn<TContext> =
    Box<dyn Fn(TContext, serde_json::Value) -> Pin<Box<dyn Future<Output = serde_json::Value>>>>;

impl<TContext> IntoHandler for Router<TContext> {
    type Handler = DynHandlerFn<TContext>;
}

#[tokio::main]
async fn main() {
    // We declare this within `main` to ensure it's never directly hardcoded in the traits above.
    // You don't need to copy this.
    pub struct Context {
        // This could hold your database connection or global configuration.
    }

    let router = Router::<Context>::new()
        .mount("sync", |_ctx, _: ()| "Hello, World Sync!")
        .mount("async", |_ctx, _: ()| async move { "Hello, World Async!" })
        .mount("echo", |_ctx, arg: String| arg);

    // Export types
    // TODO

    // Execute actions on the router
    // You could hook this up with Axum, Tauri, gRPC, or literally anything else.
    {
        let route = router.routes().get("sync").expect("Failed to get 'sync'");
        let result = (route.exec)(Context {}, json!(null)).await;
        assert_eq!(result, json!("Hello, World Sync!"));
        println!("{result:?}");
    }

    {
        let route = router.routes().get("async").expect("Failed to get 'async'");
        let result = (route.exec)(Context {}, json!(null)).await;
        assert_eq!(result, json!("Hello, World Async!"));
        println!("{result:?}");
    }

    {
        let route = router.routes().get("echo").expect("Failed to get 'echo'");
        let result = (route.exec)(Context {}, json!("Hello World Echo!")).await;
        assert_eq!(result, json!("Hello World Echo!"));
        println!("{result:?}");
    }
}
