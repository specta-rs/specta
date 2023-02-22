// /// Alpha: [OpenAPI](https://www.openapis.org) language exporter.
// #[cfg(feature = "openapi")]
// #[cfg_attr(docsrs, doc(cfg(feature = "openapi")))]
// pub mod openapi;

/// [TypeScript](https://www.typescriptlang.org) language exporter.
#[cfg(feature = "typescript")]
#[cfg_attr(docsrs, doc(cfg(feature = "typescript")))]
pub mod ts;

// /// [Rust](https://www.rust-lang.org) language exporter.
// #[cfg(feature = "rust")]
// #[cfg_attr(docsrs, doc(cfg(feature = "rust")))]
// pub mod rust;

// /// [Swift](https://www.swift.org) language exporter.
// #[cfg(feature = "swift")]
// #[cfg_attr(docsrs, doc(cfg(feature = "swift")))]
// pub mod swift;

// /// [Kotlin](https://kotlinlang.org) language exporter.
// #[cfg(feature = "kotlin")]
// #[cfg_attr(docsrs, doc(cfg(feature = "kotlin")))]
// pub mod kotlin;

// /// [Go Lang](https://go.dev) language exporter.
// #[cfg(feature = "go")]
// #[cfg_attr(docsrs, doc(cfg(feature = "go")))]
// pub mod go;

macro_rules! primitive_def {
    ($($t:ident)+) => {
        $(PrimitiveType::$t)|+
    }
}

pub(crate) use primitive_def;
