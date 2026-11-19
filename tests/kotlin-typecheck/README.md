# Kotlin typecheck harness

`specta-kotlin`'s snapshots pin the generated text, but nothing compiles that text or decodes
Serde JSON with it. This harness closes that gap for the shapes covered by
[specta-rs/specta#630](https://github.com/specta-rs/specta/issues/630):

- `Serialization::None` compiles, with and without `unknown_variants`, including shapes
  Kotlinx cannot express (untagged, generic and tuple declarations).
- `Serialization::Kotlinx` compiles with and without `unknown_variants`.
- Every configuration is compiled with `kotlinc -Werror`, so a new warning fails CI.
- Hand-written samples for every Serde tagging style Kotlinx can represent (external, internal
  and adjacent) are serialized with `serde_json` and decoded and re-encoded by the generated
  Kotlin serializers. The re-encoded JSON must equal the Serde payload, key order aside.
- Unknown variants round-trip under `unknown_variants`: a newer top-level variant, a variant
  nested inside a known one, and an unknown field inside a struct payload. Without the option,
  the same payloads must be rejected.

## Running

Requires Java 17 or newer and a Kotlin command-line compiler (1.9 or newer) on `PATH`, or
`KOTLINC` pointing at one. `KOTLIN_HOME` may point at the compiler distribution if the plugin
cannot be located automatically. On macOS, `brew install kotlin` is enough.

```sh
tests/kotlin-typecheck/run.sh
```

The script downloads `kotlinx-serialization-core` and `kotlinx-serialization-json` into `lib/`
on first use. Generated sources, downloaded jars and compiler output are ignored by Git.

## Layout

- `tests/examples/kotlin_typecheck.rs` defines the fixtures and the `serde_json` samples, and
  writes `generated/` plus `generated/samples.json`.
- `Known.kt` round-trips the known samples against the Kotlinx output without
  `unknown_variants` and asserts unknown payloads are rejected.
- `Unknown.kt` round-trips everything against the Kotlinx output with `unknown_variants` and
  asserts a top-level unknown decodes to the generated `Unknown` variant.
- `Support.kt` loads the samples and compares JSON trees.

Adding a fixture means registering it in `kotlinx_types`, adding samples, and adding the type
to the `when` blocks in `Known.kt` and `Unknown.kt` if it should be decoded by the harness.
