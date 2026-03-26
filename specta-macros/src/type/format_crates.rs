//! Format parser crate discovery for `#[derive(specta::Type)]`.
//!
//! Specta macros discover format parser crates at macro-expansion time and
//! invoke each parser crate's `parser!` macro for container/variant/field
//! attributes.
//!
//! Discovery sources, in order:
//! 1. `SPECTA_FORMAT_CRATES`, a comma-separated list provided by the build
//!    environment (for example from `build.rs`).
//!
//! A common setup in `build.rs`:
//! ```rust,ignore
//! fn main() {
//!     println!("cargo:rustc-env=SPECTA_FORMAT_CRATES=my-format-parser,another-parser");
//! }
//! ```
//!
//! Entry resolution rules:
//! - Each entry is first resolved as a Cargo package name using
//!   `proc_macro_crate` (rename-safe).
//! - If package resolution fails, the entry is treated as a Rust path.
//!
//! Duplicate entries are removed while preserving insertion order. The canonical
//! key for deduplication is the tokenized path string.
//!
//! Resolution is cached per crate context to avoid repeated work across macro
//! invocations. The cache key is:
//! - `CARGO_MANIFEST_DIR`
//! - the normalized `SPECTA_FORMAT_CRATES` value

use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock},
};

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::ToTokens;

const FORMAT_CRATES_ENV_VAR: &str = "SPECTA_FORMAT_CRATES";

static FORMAT_CRATE_CACHE: OnceLock<Mutex<HashMap<CacheKey, CacheValue>>> = OnceLock::new();

type CacheValue = Result<Vec<String>, String>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    manifest_dir: String,
    env_value: String,
}

impl CacheKey {
    fn from_env() -> Self {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let env_value = std::env::var(FORMAT_CRATES_ENV_VAR).unwrap_or_default();

        Self {
            manifest_dir,
            env_value,
        }
    }
}

pub(crate) fn format_crates() -> syn::Result<Vec<syn::Path>> {
    cached_format_crate_strings()?
        .into_iter()
        .map(|path| {
            syn::parse_str::<syn::Path>(&path).map_err(|_| {
                syn::Error::new(
                    Span::call_site(),
                    format!(
                        "specta: internal error: cached path `{path}` is not a valid Rust path"
                    ),
                )
            })
        })
        .collect()
}

fn cached_format_crate_strings() -> syn::Result<Vec<String>> {
    let key = CacheKey::from_env();
    let cache = FORMAT_CRATE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    let lock_error = |context: &str| {
        syn::Error::new(
            Span::call_site(),
            format!("specta: format crate cache lock poisoned while {context}"),
        )
    };

    if let Some(cached) = cache
        .lock()
        .map_err(|_| lock_error("reading"))?
        .get(&key)
        .cloned()
    {
        return cached.map_err(|err| syn::Error::new(Span::call_site(), err));
    }

    let computed = compute_format_crate_strings();
    let cache_entry = computed.clone().map_err(|err| err.to_string());

    cache
        .lock()
        .map_err(|_| lock_error("writing"))?
        .insert(key, cache_entry);

    computed
}

fn compute_format_crate_strings() -> syn::Result<Vec<String>> {
    let mut crates = Vec::new();

    for entry in std::env::var(FORMAT_CRATES_ENV_VAR)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let crate_path = match resolve_package_crate_path(entry)? {
            Some(path) => path,
            None => syn::parse_str::<syn::Path>(entry).map_err(|_| {
                syn::Error::new(
                    Span::call_site(),
                    format!(
                        "specta: invalid crate entry `{entry}` in ${FORMAT_CRATES_ENV_VAR}. Use a package name or a valid Rust path."
                    ),
                )
            })?,
        };

        crates.push(crate_path);
    }

    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(crates.len());
    for path in crates {
        let path = path.to_token_stream().to_string();
        if seen.insert(path.clone()) {
            normalized.push(path);
        }
    }

    Ok(normalized)
}

fn resolve_package_crate_path(package_name: &str) -> syn::Result<Option<syn::Path>> {
    let resolved = match crate_name(package_name) {
        Ok(resolved) => resolved,
        Err(proc_macro_crate::Error::CrateNotFound { .. }) => return Ok(None),
        Err(err) => {
            return Err(syn::Error::new(
                Span::call_site(),
                format!("specta: failed to resolve crate `{package_name}`: {err}"),
            ));
        }
    };

    let path = match resolved {
        FoundCrate::Itself => syn::parse_quote!(crate),
        FoundCrate::Name(name) => syn::parse_str(&name).map_err(|_| {
            syn::Error::new(
                Span::call_site(),
                format!("specta: resolved crate name `{name}` is not a valid Rust path"),
            )
        })?,
    };

    Ok(Some(path))
}
