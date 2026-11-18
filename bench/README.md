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
| `expanded_bytes`   | bytes of `-Zunpretty=expanded`; deterministic, tracks code volume  |
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

Baseline is the parent commit, measured back to back with the final tree in the
same session (min of 11 runs):

| metric             | before    | after     |         |
| ------------------ | --------- | --------- | ------- |
| `expanded_bytes`   | 6,643,411 | 4,429,643 | −33.3 % |
| `expand_crate`     | 0.286 s   | 0.187 s   | −34.6 % |
| `type_check_crate` | 0.501 s   | 0.354 s   | −29.3 % |
| `codegen_crate`    | 0.354 s   | 0.132 s   | −62.7 % |
| `total`            | 2.875 s   | 1.760 s   | −38.8 % |
