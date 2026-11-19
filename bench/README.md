# Derive benchmark harness

Measures the compile-time cost of `#[derive(Type)]`. Not wired into CI and not
part of the repository workspace: `bench/derive-perf` and `bench/serde-shim`
each declare an empty `[workspace]` table, so `cargo` at the repository root
ignores them.

## Setup

```sh
python3 bench/gen.py            # writes bench/derive-perf/src/lib.rs (gitignored)
python3 bench/measure.py <label> [iterations]
```

`gen.py` emits ~1250 derives (`SCALE=4`), sampling the shapes the macro has to
handle: plain and tuple structs, newtypes, enums with all four variant shapes,
generic structs/enums with bounds, defaults and `skip_default_generic`, serde
metadata (`rename_all`, `tag`, `flatten`, `skip`, `with`, `transparent`,
deprecated containers, docs on everything) and
`#[specta(type|inline|optional)]` overrides.

`serde-shim` is a no-op derive that only registers `attributes(serde)`, so the
fixture can carry serde metadata without pulling `serde_derive` into the
measurement.

## Metrics

| metric             | what it is                                                        |
| ------------------ | ----------------------------------------------------------------- |
| `expanded_bytes`   | bytes of `-Zunpretty=expanded`; deterministic                      |
| `expanded_tokens`  | the same with all whitespace removed — a proxy for real tokens     |
| `expand_crate`     | rustc pass time for all macro expansion                           |
| `type_check_crate` | downstream type-check of the expanded code                        |
| `codegen_crate`    | downstream codegen                                                |
| `wall`/`cpu`       | whole `cargo rustc` process                                       |

`measure.py` reports the minimum and median of N runs; compare the minimum — the
machine is busy enough that medians drift a few percent between runs.

`~/.cargo/config.toml` on this machine routes rustc through `kache`, which
replays cached stderr, so repeated runs come back byte-identical. `measure.py`
sets `CARGO_BUILD_RUSTC_WRAPPER=""`; do the same when running rustc by hand.

## Results

Baseline is the commit the branch started from (`10ab28a9`), measured back to
back with the final tree in the same session (min of 13 runs):

| metric             | before    | after     |          |
| ------------------ | --------- | --------- | -------- |
| `expanded_bytes`   | 6,643,411 | 2,674,466 | −59.7 %  |
| `expanded_tokens`  | 3,065,904 | 1,554,770 | −49.3 %  |
| `expand_crate`     | 0.294 s   | 0.180 s   | −38.8 %  |
| `type_check_crate` | 0.522 s   | 0.278 s   | −46.7 %  |
| `codegen_crate`    | 0.359 s   | 0.109 s   | −69.6 %  |
| `total`            | 2.981 s   | 1.390 s   | −53.4 %  |

`expanded_tokens` still counts long identifiers as one token each, so it
overstates the compiler's work when a change only shortens names; `expand_crate`
is the number that matters for the macro itself.
