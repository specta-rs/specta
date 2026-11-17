//! Build script for configuring nightly-only cfg flags.
//!
//! This build script enables the `is_nightly` cfg when running on a nightly
//! compiler (or when `RUSTC_BOOTSTRAP` is set) so crate code can conditionally
//! compile nightly-specific behavior.
//!
//! It would be nice if this was built into Cargo 😅

fn main() {
    let is_nightly = std::env::var("RUSTC_BOOTSTRAP").is_ok()
        || rustc_version::version_meta()
            .map(|m| m.channel == rustc_version::Channel::Nightly)
            .unwrap_or(false);

    if is_nightly {
        println!("cargo:rustc-cfg=is_nightly");
    }
}
