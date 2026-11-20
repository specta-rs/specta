//! Allocation harness for the `Type` derive.
//!
//! Run with:
//!
//! ```sh
//! RUSTFLAGS="-C prefer-dynamic=no" \
//!   cargo test -p specta-macros --lib -- --ignored --nocapture alloc
//! ```
//!
//! The `RUSTFLAGS` is not optional. A proc-macro crate's unit test links
//! `libstd` dynamically, and a dylib's `Vec`/`String` allocations go through
//! its own copy of the allocator shim instead of the one this crate installs:
//! without the flag only `Box`/`Rc`-style allocations are counted, which is the
//! wrong half of the picture. (The harness checks this and prints a warning.)
//!
//! It is `#[ignore]`d so it never runs in normal CI: it is a diagnostic, not an
//! assertion. The fixture is generated in this file rather than checked in, so
//! it stays readable and changes with the shapes the derive has to handle.
//!
//! See [`crate::alloc`] for the measuring machinery and the warning about
//! absolute numbers: a test links `proc-macro2`'s fallback, so only compare two
//! builds of this crate against each other.
//!
//! "Expansion" here is `derive_tokens` minus the parse it does internally (both
//! parse the identical tokens with `syn`, so the difference is exactly the
//! derive's own work). The lexing of the source is measured separately as
//! "tokenize"; the real macro bridge hands the derive tokens it did not lex, so
//! neither of the other two columns includes it.

use proc_macro2::TokenStream;
use syn::DeriveInput;

use crate::alloc::{Entry, Measured, measure, take_log};

use super::derive_tokens;

/// One derive's cost, split into the three phases.
struct Sample {
    tokenize: Measured,
    parse: Measured,
    expansion: Measured,
    /// Identifier tokens in the expansion. Each one is an owned string in
    /// `proc-macro2`'s fallback, so an expansion cannot cost fewer allocations
    /// than this in this harness.
    idents: usize,
    /// The expansion's own attribution, one entry per closed phase guard.
    phases: Vec<Entry>,
}

/// How many identifier tokens `tokens` contains, at any nesting depth.
fn count_idents(tokens: &TokenStream) -> usize {
    tokens
        .clone()
        .into_iter()
        .map(|tree| match tree {
            proc_macro2::TokenTree::Ident(_) => 1,
            proc_macro2::TokenTree::Group(group) => count_idents(&group.stream()),
            _ => 0,
        })
        .sum()
}

fn sample(src: &str) -> Sample {
    let (tokens, tokenize) = measure(|| src.parse::<TokenStream>().expect("fixture should lex"));

    // The fixture is also what the compiler would hand the derive, so make sure
    // it parses as a `DeriveInput` up front: a broken fixture should fail the
    // harness with a message that names it, not silently measure the error path.
    assert!(
        syn::parse_str::<DeriveInput>(src).is_ok(),
        "fixture should parse: {src}"
    );

    let (_, parse) =
        measure(|| syn::parse2::<DeriveInput>(tokens.clone()).expect("fixture should parse"));
    // Only the expansion's own guards belong in its attribution; the tokenize
    // and parse measurements above opened a guard of their own.
    let _ = take_log();

    let (expanded, total) = measure(|| derive_tokens(tokens, "0").expect("derive should succeed"));
    assert!(!expanded.is_empty(), "derive produced nothing for {src}");
    let phases = take_log();
    let idents = count_idents(&expanded);

    Sample {
        tokenize,
        parse,
        expansion: total.saturating_sub(parse),
        idents,
        phases,
    }
}

fn mean(total: Measured, samples: usize) -> Measured {
    Measured {
        allocations: total.allocations / samples,
        bytes: total.bytes / samples,
    }
}

#[test]
#[ignore = "diagnostic, not an assertion; run with --ignored --nocapture"]
fn alloc_report() {
    let cases = fixture();

    // (kind, count, expansion, idents)
    let mut by_kind: Vec<(&'static str, usize, Measured, usize)> = Vec::new();
    let mut tokenize_total = Measured::default();
    let mut parse_total = Measured::default();
    let mut expansion_total = Measured::default();
    let mut idents_total = 0;
    // (label, occurrences, inclusive, own)
    let mut by_phase: Vec<(&'static str, usize, Measured, Measured)> = Vec::new();
    // Work done between the instrumented regions.
    let mut unattributed = Measured::default();

    for (kind, src) in &cases {
        let sample = sample(src);
        tokenize_total = tokenize_total.checked_add(sample.tokenize);
        parse_total = parse_total.checked_add(sample.parse);
        expansion_total = expansion_total.checked_add(sample.expansion);
        idents_total += sample.idents;

        match by_kind.iter_mut().find(|(name, ..)| name == kind) {
            Some((_, count, total, idents)) => {
                *count += 1;
                *total = total.checked_add(sample.expansion);
                *idents += sample.idents;
            }
            None => by_kind.push((kind, 1, sample.expansion, sample.idents)),
        }

        for entry in &sample.phases {
            if entry.label == "unattributed" {
                unattributed = unattributed.checked_add(entry.own);
                continue;
            }

            match by_phase
                .iter_mut()
                .find(|(label, ..)| *label == entry.label)
            {
                Some((_, count, inclusive, own)) => {
                    *count += 1;
                    *inclusive = inclusive.checked_add(entry.inclusive);
                    *own = own.checked_add(entry.own);
                }
                None => by_phase.push((entry.label, 1, entry.inclusive, entry.own)),
            }
        }
    }

    let n = cases.len();
    println!();
    println!("== `#[derive(Type)]` allocations (proc-macro2 fallback) ==");
    println!();

    // Two bytes on the heap is the smallest `Vec` allocation there is. If it is
    // not seen, `libstd` is linked dynamically and `Vec`/`String` allocations
    // bypass this crate's allocator, so every number below is incomplete.
    let (_, probe) = measure(|| vec![0u8; 2]);
    if probe.allocations == 0 {
        println!("!! Vec/String allocations are NOT being counted.");
        println!("!! Re-run with RUSTFLAGS=\"-C prefer-dynamic=no\" (see this file's doc).");
        println!();
    }

    println!(
        "{:<24} {:>4} {:>14} {:>13} {:>13}",
        "phase", "n", "allocs/derive", "bytes/derive", "total allocs"
    );
    println!("{}", "-".repeat(72));
    for (label, measured) in [
        ("tokenize (source)", tokenize_total),
        ("parse (syn)", parse_total),
        ("expansion", expansion_total),
    ] {
        let mean = mean(measured, n);
        println!(
            "{label:<24} {n:>4} {:>14} {:>13} {:>13}",
            mean.allocations, mean.bytes, measured.allocations
        );
    }
    // An ident has an owned string in this harness, so one allocation per
    // emitted ident is the floor; everything above it is copying, `Vec` growth
    // and the strings the derive itself builds.
    println!(
        "the expansion emits {:.1} idents per derive, so {:.1} of the {:.1} allocations are its \
         floor.",
        idents_total as f64 / n as f64,
        idents_total as f64 / n as f64,
        expansion_total.allocations as f64 / n as f64,
    );
    println!();

    // `inclusive` counts everything that happened while the guard was open, so
    // rows nest and their sum exceeds the phase total; `own` subtracts the
    // children out. `own` for the outermost instrumented phase is what is left
    // of the expansion once every named phase is accounted for.
    println!("expansion attribution (per derive)");
    print_phases(&mut by_phase, n);

    // Every guard reports the work `own` (children subtracted), so the rows
    // partition the expansion; whatever is left over happened between the
    // measured regions.
    println!(
        "{} per derive are outside every measured region",
        unattributed.allocations / n
    );
    println!();

    println!("per shape (expansion only)");
    println!("{}", "-".repeat(74));
    println!(
        "{:<24} {:>4} {:>11} {:>8} {:>8} {:>9}",
        "kind", "n", "allocs/der", "idents", "floor", "overhead"
    );
    for (kind, count, total, idents) in &by_kind {
        let mean = mean(*total, *count);
        println!(
            "{kind:<24} {count:>4} {:>11} {:>8.1} {:>8} {:>9}",
            mean.allocations,
            *idents as f64 / *count as f64,
            mean.allocations - idents / count,
            idents / count,
        );
    }
    println!();

    // Attribution for the per-derive floor and the per-thing slopes. These are
    // not part of the sample; they are the smallest inputs that isolate one
    // cost, which is what makes a candidate refactor's worth visible.
    println!("attribution (expansion allocations)");
    println!("{}", "-".repeat(74));
    for (label, src) in [
        ("unit struct", "struct A;"),
        ("named struct, 1 field", "struct A { a: String }"),
        (
            "named struct, 4 fields",
            "struct A { a: String, b: u32, c: bool, d: f64 }",
        ),
        (
            "named struct + 1 serde attr",
            "#[serde(rename_all = \"camelCase\")] struct A { a: String }",
        ),
        (
            "named struct + 3 attrs",
            "#[serde(rename_all = \"camelCase\")] struct A { #[serde(rename = \"b\")] a: String, #[specta(optional)] b: String }",
        ),
        ("enum, 4 unit variants", "enum A { A, B, C, D }"),
        (
            "enum, 4 named variants",
            "enum A { A { a: String }, B { a: String }, C { a: String }, D { a: String } }",
        ),
        ("generic struct, 1 param", "struct A<T> { a: T, b: Vec<T> }"),
        (
            "generic struct, 2 bounded params",
            "struct A<T: Clone, U: Send> where U: Sync { a: T, b: Vec<U>, c: Option<T> }",
        ),
        (
            "generic struct, default param",
            "struct A<T = String, U = T> { a: T, b: U }",
        ),
        ("raw ident field", "struct A { r#type: String }"),
    ] {
        let expansion = sample(src).expansion;
        println!(
            "{label:<36} {:>8} allocs {:>8} bytes",
            expansion.allocations, expansion.bytes
        );
    }
    println!();
}

/// Print the attribution table. `n` is the number of derives the tallies cover.
///
/// `inclusive` counts everything that happened while the guard was open, so rows
/// nest and their sum exceeds the expansion total; `own` subtracts the children
/// out. An allocation is counted exactly once across the `own` column.
fn print_phases(by_phase: &mut [(&'static str, usize, Measured, Measured)], n: usize) {
    by_phase.sort_by_key(|(_, _, inclusive, _)| std::cmp::Reverse(inclusive.allocations));
    println!("{}", "-".repeat(78));
    println!(
        "{:<24} {:>9} {:>12} {:>12} {:>12}",
        "phase", "per derive", "incl/derive", "own/derive", "bytes/derive"
    );
    for (label, count, inclusive, own) in by_phase {
        let per_derive = *count as f64 / n as f64;
        println!(
            "{label:<24} {per_derive:>9.2} {:>12} {:>12} {:>12}",
            inclusive.allocations / n,
            own.allocations / n,
            own.bytes / n
        );
    }
    println!();
}

/// ~120 definitions covering the shapes the derive lowers.
fn fixture() -> Vec<(&'static str, String)> {
    let mut cases = Vec::new();

    for n in 0..10 {
        cases.push((
            "struct: named",
            format!(
                "pub struct Named{n} {{ pub a: String, pub b: u32, pub c: Option<Vec<bool>> }}"
            ),
        ));
        cases.push((
            "struct: tuple",
            format!("pub struct Tuple{n}(pub String, pub u32, pub bool);"),
        ));
        cases.push((
            "struct: newtype",
            format!("pub struct Newtype{n}(pub String);"),
        ));
        cases.push(("struct: unit", format!("pub struct Unit{n};")));

        cases.push(("enum: unit", format!("pub enum Units{n} {{ A, B, C, D }}")));
        cases.push((
            "enum: newtype",
            format!("pub enum Newtypes{n} {{ A(String), B(u32), C(Vec<bool>), D(Option<u8>) }}"),
        ));
        cases.push((
            "enum: tuple",
            format!(
                "pub enum Tuples{n} {{ A(String, u32), B(bool, u8, f64), C(u8, u8), D(String, String) }}"
            ),
        ));
        cases.push((
            "enum: named",
            format!(
                "pub enum Nameds{n} {{ A {{ x: String, y: u32 }}, B {{ z: bool }}, C {{ a: String, b: u8 }}, D {{ q: Option<String> }} }}"
            ),
        ));

        cases.push((
            "generic: struct",
            format!(
                "pub struct GenStruct{n}<T: Clone + Send, U = String> where U: Sync {{ pub a: T, pub b: Vec<U>, pub c: Option<T> }}"
            ),
        ));
        cases.push((
            "generic: enum",
            format!(
                "pub enum GenEnum{n}<'a, T: Clone, const N: usize> {{ A(&'a T), B([u8; N]), C {{ value: T }} }}"
            ),
        ));

        cases.push((
            "metadata",
            format!(
                r#"
/// Documented thing.
#[deprecated(since = "1.2.3", note = "use something else")]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Meta{n}<T> {{
    /// A variant.
    #[serde(rename = "first")]
    First {{
        /// a field
        #[serde(rename = "someField", default)]
        some_field: Option<T>,
        #[serde(flatten)]
        rest: String,
    }},
    #[specta(skip)]
    Skipped,
    #[specta(type = String)]
    Overridden(u32),
}}
"#
            ),
        ));

        cases.push((
            "specta attrs",
            format!(
                r#"
#[specta(collect = false, bound = "T: Clone")]
pub struct SpectaAttrs{n}<T> {{
    pub r#type: T,
    #[specta(optional)]
    pub maybe: Option<String>,
    #[specta(skip)]
    pub hidden: std::marker::PhantomData<T>,
}}
"#
            ),
        ));

        cases.push((
            "specta transparent",
            format!("#[specta(transparent)]\npub struct Transparent{n}(pub Vec<String>);"),
        ));
    }

    cases
}

/// What the primitives the derive builds its bookkeeping from cost.
///
/// A derive is mostly `quote!` calls, `Ident` construction and `Path`s, so
/// knowing what one of each costs is what makes the report's per-shape numbers
/// attributable: e.g. a `parse_quote!` is ~13 allocations, which is why the
/// three aliases in `Names` are built with `Ident::new` instead.
#[test]
#[ignore = "diagnostic; run with --ignored --nocapture"]
fn alloc_primitives() {
    use quote::{format_ident, quote};

    let ident = format_ident!("Type");
    let path: syn::Path = syn::parse_quote!(specta);

    println!();
    println!("primitive costs (allocations / bytes, one call)");
    println!("{}", "-".repeat(56));
    for (label, measured) in [
        ("quote!()", measure(|| quote!()).1),
        ("quote!(#ident)", measure(|| quote!(#ident)).1),
        ("quote!(#path)", measure(|| quote!(#path)).1),
        (
            "quote! with 3 interpolations",
            measure(|| quote!(impl #path::Type for #ident {})).1,
        ),
        (
            "format_ident!(\"__P{}\", 0)",
            measure(|| format_ident!("__P{}", 0usize)).1,
        ),
        (
            "Ident::new",
            measure(|| proc_macro2::Ident::new("__T", proc_macro2::Span::call_site())).1,
        ),
        (
            "syn::Path::from(Ident)",
            measure(|| syn::Path::from(ident.clone())).1,
        ),
        (
            "parse_quote!(specta)",
            measure(|| -> syn::Path { syn::parse_quote!(specta) }).1,
        ),
        ("Ident::to_string", measure(|| ident.to_string()).1),
        (
            "syn::parse2::<DeriveInput>",
            measure(|| {
                syn::parse2::<DeriveInput>(quote!(
                    struct A;
                ))
            })
            .1,
        ),
    ] {
        println!(
            "{label:<32} {:>4} allocs {:>7} bytes",
            measured.allocations, measured.bytes
        );
    }
    println!();
}

/// Attribution for one shape at a time, so a candidate refactor can be priced
/// against the shape that pays for it instead of the 130-derive average.
#[test]
#[ignore = "diagnostic; run with --ignored --nocapture alloc_detail"]
fn alloc_detail() {
    let shapes = [
        ("unit struct", "struct A;"),
        (
            "named struct, 4 fields",
            "struct A { a: String, b: u32, c: bool, d: f64 }",
        ),
        (
            "named enum, 4 named variants",
            "enum A { A { a: String }, B { a: String }, C { a: String }, D { a: String } }",
        ),
        ("generic struct, 1 param", "struct A<T> { a: T, b: Vec<T> }"),
        (
            "generic struct, 2 bounded params",
            "struct A<T: Clone, U: Send> where U: Sync { a: T, b: Vec<U>, c: Option<T> }",
        ),
        (
            "specta attrs",
            r#"
#[specta(collect = false, bound = "T: Clone")]
pub struct A<T> {
    pub r#type: T,
    #[specta(optional)]
    pub maybe: Option<String>,
    #[specta(skip)]
    pub hidden: std::marker::PhantomData<T>,
}
"#,
        ),
        (
            "generic enum",
            "enum A<'a, T: Clone, const N: usize> { A(&'a T), B([u8; N]), C { value: T } }",
        ),
        (
            "metadata enum",
            r#"
#[serde(rename_all = "camelCase", tag = "kind")]
enum A<T> {
    #[serde(rename = "first")]
    First {
        #[serde(rename = "someField", default)]
        some_field: Option<T>,
        #[serde(flatten)]
        rest: String,
    },
    #[specta(skip)]
    Skipped,
    #[specta(type = String)]
    Overridden(u32),
}
"#,
        ),
    ];

    println!();
    {
        let sample = sample("struct A;");
        println!(
            "DEBUG labels: {:?}",
            sample
                .phases
                .iter()
                .map(|e| (e.label, e.own.allocations))
                .collect::<Vec<_>>()
        );
    }
    for (label, src) in shapes {
        let sample = sample(src);
        let mut by_phase: Vec<(&'static str, usize, Measured, Measured)> = Vec::new();
        for entry in &sample.phases {
            match by_phase.iter_mut().find(|(name, ..)| *name == entry.label) {
                Some((_, count, inclusive, own)) => {
                    *count += 1;
                    *inclusive = inclusive.checked_add(entry.inclusive);
                    *own = own.checked_add(entry.own);
                }
                None => by_phase.push((entry.label, 1, entry.inclusive, entry.own)),
            }
        }

        println!(
            "== {label}: {} allocs, {} bytes ==",
            sample.expansion.allocations, sample.expansion.bytes
        );
        print_phases(&mut by_phase, 1);
    }
}
