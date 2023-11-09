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
        // This module seal's the private fields of the `$router` struct.
        $crate::internal::paste! {
            #[allow(non_snake_case)]
            mod [<$router _priv>] {
                pub use super::*;
                pub use std::{collections::HashMap, borrow::Cow, marker::PhantomData};

                pub struct Route<H> {
                    // TODO: Types

                    exec: H
                }

                pub struct $router<$($router_generic),*> {
                    routes: HashMap<Cow<'static, str>, Route<<Self as $crate::IntoHandler>::Handler>>,
                    phantom: PhantomData<fn() -> ($($router_generic,)*)>
                }

                impl<$($router_generic),*> $router<$($router_generic),*>
                    where $($y: $x $(::$xx)* $(+ $xxx $(::$xxxx)*)*),*
                {
                    pub fn new() -> Self {
                        Self {
                            routes: HashMap::new(),
                            phantom: PhantomData,
                        }
                    }

                    pub fn mount<H $(, $handler_generics)*>(
                        mut self,
                        name: impl Into<std::borrow::Cow<'static, str>>,
                        handler: H,
                    ) -> Self where $($handler_bound)* {
                        // TODO: Store type
                        // TODO: Store user-defined generics

                        self.routes.insert(name.into(), Route {
                            exec: todo!(),
                        });

                        self
                    }

                    // TODO: Iterator

                    // TODO: `.merge`

                    // TODO: `.build()`???
                }
            }

            pub use [<$router _priv>]::$router;
        }
    };
}
