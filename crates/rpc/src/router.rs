#[macro_export]
macro_rules! router {
    ($router:ident [ $handler:ident $(, $generic:ident)* ]) => {
        // $crate::router!($router::<$handler $(, $generic:ident)*> where);
    };
    ($router:ident::< $($router_generic:ident),* > [ $handler:ident $(, $generic:ident)* ]) => {
        // $crate::router!($router::<$handler $(, $generic:ident)*> where);
    };
    ($router:ident [ $handler:ident $(, $generic:ident)* ] where $($bound:tt)*) => {
        // $crate::router!($router::<$handler $(, $generic:ident)*> where);
    };
    ($router:ident::< $($router_generic:ident),* > [ $handler:ident $(, $generic:ident)* ] where $($bound:tt)*) => {
        // $crate::router!($router::<$handler $(, $generic:ident)*> where);
    }; // ($router:ident < $handler:ident $(, $generic:ident)* > where $($bound:tt)*) => {
       //     $crate::router!($router::<> <$handler $(, $generic:ident)*> where);
       // };
       // ($router:ident::< $($router_generic:ident)* > < $handler:ident $(, $generic:ident)* > where $($bound:tt)*) => {
       //    pub struct $router {}

       //     impl $router {
       //         pub fn new() -> Self {
       //             Self {}
       //         }

       //         pub fn mount<H $(, $generic)*>(
       //             self,
       //             name: impl Into<std::borrow::Cow<'static, str>>,
       //             handler: H,
       //         ) -> Self where $($bound)* {
       //             // let y =

       //             // TODO: Store type
       //             // TODO: Store runtime version -> Boxed probally but leave it up to the user

       //             self
       //         }

       //         // TODO: `.merge`

       //         // TODO: `.build()`???
       //     }
       // };
}
