//! Build script for configuring compiler-version-dependent cfg flags.
//!
//! This build script enables:
//! - `is_nightly` when running on a nightly compiler (or with `RUSTC_BOOTSTRAP`),
//!   so crate code can conditionally compile nightly-specific behavior.
//! - `has_never_type` when the compiler has the stable never type (`!`). Rust
//!   1.100.0 stabilised it and made `std::convert::Infallible` an alias for it.
//!
//! It would be nice if this was built into Cargo 😅

use rustc_version::Channel;

fn main() {
    let meta = rustc_version::version_meta().ok();

    let is_nightly = std::env::var("RUSTC_BOOTSTRAP").is_ok()
        || meta.as_ref().is_some_and(|m| m.channel == Channel::Nightly);
    if is_nightly {
        println!("cargo:rustc-cfg=is_nightly");
    }

    // Compare major/minor so a `1.100.0-nightly` pre-release counts as 1.100.
    let has_never_type = meta.as_ref().is_some_and(|m| {
        let version = &m.semver;
        version.major > 1 || (version.major == 1 && version.minor >= 100)
    });
    if has_never_type {
        println!("cargo:rustc-cfg=has_never_type");
    }
}
