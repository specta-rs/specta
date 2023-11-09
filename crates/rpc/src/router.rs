#[macro_export]
macro_rules! router {
    // Just `mount` generics
    ($router:ident [ $handler:ident $(, $handler_generics:ident)* ]) => {
        $crate::router!($router::<> [ $handler $(, $handler_generics)* ] where);
    };
    ($router:ident [ $handler:ident $(, $handler_generics:ident)* ] where $($handler_bound:tt)*) => {
        $crate::router!($router::<> [ $handler $(, $handler_generics)* ] where);
    };
    // With router generics
    ($router:ident::< $($router_generic:ident),* > [ $handler:ident $(, $handler_generics:ident)* ]) => {
        $crate::router!($router::<$($router_generic),*> [ $handler $(, $handler_generics)* ] where);
    };
    ($router:ident::< $($router_generic:ident),* > [ $handler:ident $(, $handler_generics:ident)* ] where $($handler_bound:tt)*) => {
        $crate::router!($router::<$($router_generic),*> where; [ $handler $(, $handler_generics)* ] where $($handler_bound)*);
    };
    // With router generics and router bounds
    ($router:ident::< $($router_generic:ident),* > where $( $y:ident: $x:ident $(::$xx:ident)* $(+ $xxx:ident $(::$xxxx:ident)* )* ),*; [ $handler:ident $(, $handler_generics:ident)* ]) => {
        $crate::router!($router::<$($router_generic),*> where $($y: $x $(::$xx)* $(+ $xxx $(::$xxxx)*)*),*; [ $handler $(, $handler_generics)* ] where);
    };
    ($router:ident::< $($router_generic:ident),* > where $( $y:ident: $x:ident $(::$xx:ident)* $(+ $xxx:ident $(::$xxxx:ident)* )* ),*; [ $handler:ident $(, $handler_generics:ident)* ] where $($handler_bound:tt)*) => {
        pub struct $router<$($router_generic),*> {
            phantom: std::marker::PhantomData<fn() -> ($($router_generic,)*)>
        }

        impl<$($router_generic),*> $router<$($router_generic),*>
            where $($y: $x $(::$xx)* $(+ $xxx $(::$xxxx)*)*),*
        {
            pub fn new() -> Self {
                Self {
                    phantom: std::marker::PhantomData,
                }
            }

            pub fn mount<H $(, $handler_generics)*>(
                self,
                name: impl Into<std::borrow::Cow<'static, str>>,
                handler: H,
            ) -> Self where $($handler_bound)* {
                // TODO: Store type
                // TODO: Store runtime version -> Boxed probally but leave it up to the user

                self
            }

            // TODO: `.merge`

            // TODO: `.build()`???
        }
    };
}
