//! Specta Remote Procedure Call (RPC) library.
// TODO: Add documentation.
// TODO: Add lints

mod router;

#[doc(hidden)]
pub mod internal {
    pub use paste::paste;
}

pub trait IntoHandler {
    type Handler;
}

pub trait FromRouter<R: IntoHandler> {}
