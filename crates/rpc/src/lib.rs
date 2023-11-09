//! Specta Remote Procedure Call (RPC) library.
// TODO: Add documentation.
// TODO: Add lints

mod router;

pub trait IntoHandler {
    type Handler;
}

#[doc(hidden)]
pub mod internal {
    pub use paste::paste;
}
