#[macro_export]
macro_rules! router {
    // Just `mount` generics
    ($router:ident [ $handler:ident $(=> $a:expr)? $(, $handler_generics:ident)* ]) => {
        $crate::router!($router::<> [ $handler $(=> $a)? $(, $handler_generics)* ] where);
    };
    ($router:ident [ $handler:ident $(=> $a:expr)? $(, $handler_generics:ident)* ] where $($handler_bound:tt)*) => {
        $crate::router!($router::<> [ $handler $(=> $a)? $(, $handler_generics)* ] where);
    };
    // With router generics
    ($router:ident::< $($router_generic:ident),* > [ $handler:ident $(=> $a:expr)? $(, $handler_generics:ident)* ]) => {
        $crate::router!($router::<$($router_generic),*> [ $handler $(=> $a)? $(, $handler_generics)* ] where);
    };
    ($router:ident::< $($router_generic:ident),* > [ $handler:ident $(=> $a:expr)? $(, $handler_generics:ident)* ] where $($handler_bound:tt)*) => {
        $crate::router!($router::<$($router_generic),*> where; [ $handler $(=> $a)? $(, $handler_generics)* ] where $($handler_bound)*);
    };
    // With router generics and router bounds
    ($router:ident::< $($router_generic:ident),* > where $( $y:ident: $x:ident $(::$xx:ident)* $(+ $xxx:ident $(::$xxxx:ident)* )* ),*; [ $handler:ident $(=> $a:expr)? $(, $handler_generics:ident)* ]) => {
        $crate::router!($router::<$($router_generic),*> where $($y: $x $(::$xx)* $(+ $xxx $(::$xxxx)*)*),*; [ $handler $(=> $a)? $(, $handler_generics)* ] where);
    };
    ($router:ident::< $($router_generic:ident),* > where $( $y:ident: $x:ident $(::$xx:ident)* $(+ $xxx:ident $(::$xxxx:ident)* )* ),*; [ $handler:ident => $handler_fn:expr $(, $handler_generics:ident)* ] where $($handler_bound:tt)*) => {
        // This module seal's the private fields of the `$router` struct.
        $crate::internal::paste! {
            #[allow(non_snake_case)]
            mod [<$router _priv>] {
                pub use super::*;
                pub use std::{collections::HashMap, borrow::Cow, marker::PhantomData};

                pub struct Route<H> {
                    // TODO: Types

                    pub exec: H
                }

                pub struct $router<$($router_generic),*> {
                    routes: HashMap<Cow<'static, str>, Route<<Self as $crate::IntoHandler>::Handler>>,
                    phantom: PhantomData<fn() -> ($($router_generic,)*)>
                }

                fn assert_valid_handler_fn<H, R, F: Fn(H) -> R>(f: F) -> F {
                    f
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
                        let handler_fn = assert_valid_handler_fn::<H, <Self as $crate::IntoHandler>::Handler, _>($handler_fn);

                        // TODO: Error for duplicate `name`


                        self.routes.insert(name.into(), Route {
                            // TODO: Store type
                            // TODO: Store user-defined generics
                            exec: (handler_fn)(handler),
                        });

                        self
                    }

                    pub fn routes(&self) -> &HashMap<Cow<'static, str>, Route<<Self as $crate::IntoHandler>::Handler>> {
                        &self.routes
                    }

                    pub fn routes_mut(&mut self) -> &mut HashMap<Cow<'static, str>, Route<<Self as $crate::IntoHandler>::Handler>> {
                        &mut self.routes
                    }

                }
            }

            pub use [<$router _priv>]::$router;
        }
    };
}
