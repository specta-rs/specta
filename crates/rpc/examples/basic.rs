use std::marker::PhantomData;

use specta_rpc::router;

router!(Router17::<A, B> where A: Clone, B: Clone; [H, G] where H: Clone);

router!(Router::<Ctx> [H, M] where H: Handler<Ctx, M>);

pub trait Handler<Ctx, M> {}

pub struct Marker<T, R>(PhantomData<(T, R)>);
impl<Ctx, T, R, F: Fn(Ctx, T) -> R> Handler<Ctx, Marker<T, R>> for F {}

fn main() {
    Router17::<String, String>::new();

    let router = Router::<String>::new().mount("demo", |ctx, arg: ()| "Hello, World!");
    // .mount("demo", |ctx, arg: ()| async move { "Hello, World!" });
}

macro_rules! demo {
    () => {};
    // ($($i:ident: $($x:ty)+*),*) => {};

    // ($($x:ident)::* +) => {};
    // ($( $x:ident $(::$xx:ident)*)? $(+ $xxx:ident $(::$xxxx:ident)*)? ),*) => {};
    ($( $y:ident: $x:ident $(::$xx:ident)* $(+ $xxx:ident $(::$xxxx:ident)* )* ),*) => {};
}

demo!(A: Debug);
demo!(A: std::clone::Clone);
demo!(A: Debug + std::clone::Clone);
demo!(A: Debug + Clone + Display);

demo!(A: Debug, B: Debug);
demo!(A: std::clone::Clone);
demo!(A: std::clone::Clone, B: std::clone::Clone);
demo!(A: Debug + std::clone::Clone, B: Debug + std::clone::Clone);
demo!(A: Debug + Clone + Display, B: Debug + Clone + Display);
