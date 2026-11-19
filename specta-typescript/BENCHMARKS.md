# TypeScript export performance

## Pass 1: prove acyclic aliases once per batch

Previously, every enum export ran the generic-aware alias-cycle collapse walk.
That walk clones variants and builds a reachable graph even when no cycle
exists. A chain of N untagged aliases repeated that work N times.

The exporter now borrows bodies to discover a graph with dense node indices.
Reverse topological sink removal proves which declarations cannot reach a
cycle, and a batch-local identity cache reuses those proofs. Generic declarations
are barriers: they and their predecessors still use the existing substitution
and collapse algorithm. Cyclic bodies are never cached across roots. Enums
without a transparent first hop skip graph construction, and a direct first-hop
self-cycle goes straight to collapse. Records do not allocate an analysis cache.
The transient graph borrows module/name strings. The cache also remembers alias
identities so a modified clone passed to `primitives::export` cannot hide a
name-based back edge inside an already proved tail. This changes neither the
public API nor generated output.

The proof cache is local to `primitives::export_internal`, so separate namespace
or file batches can repeat discovery. Truly cyclic and generic-sensitive graphs
still use the old collapse walk. Sink removal deliberately proves less than a
full SCC decomposition: nodes leading into a cycle retain the old analysis too.

## Reproduce

Run from the repository root (no extra dependencies or benchmark framework):

```sh
cargo bench -p specta-typescript --bench export
cargo bench -p specta-typescript --bench export -- chain/
SPECTA_BENCH_MS=500 cargo bench -p specta-typescript --bench export -- prepared
```

Each case reports the median, minimum, and maximum of seven sample means in
microseconds/export. A sample runs for at least `SPECTA_BENCH_MS` milliseconds
(default 150); slow cases can exceed that with one export. Fixture construction,
Serde preparation for `prepared-flat`, output hashing, and equality checks are
outside timed regions. Timed work includes producing and dropping the output
string. Every case performs an untimed warmup and checks repeat output equality.

Workloads:

- `records`: 32/256/1024 four-field records with nullable named predecessors.
- `enums`: 32/256/1024 independent externally tagged two-variant enums.
- `chain`: 32/128/512/1024 untagged unions with one named successor and a null
  terminal. This isolates repeated acyclic alias analysis.
- `api`: 8/128/512 response records using shared generic pages, users, addresses,
  tagged events, string maps, lists, and nullable fields.
- `cycle`: 1/8/32/128 untagged unions forming an actual alias cycle, to exercise
  the unchanged collapse fallback rather than only its fast path.
- `generic-api`: 8/128/512 response records using a shared recursive generic
  untagged enum, to exercise generic-sensitive fallback.

Each shape runs end-to-end with `specta_serde::Format` in flat TypeScript,
namespace TypeScript, and flat JSDoc layouts. `prepared-flat` applies Serde once
outside the measurement and uses a no-op graph format, measuring TypeScript
rendering separately. Its per-datatype hook returns an unchanged clone, preserving
the format contract. Output byte lengths and FNV-1a checksums allow comparison
between revisions; checksums are evidence, not a substitute for tests.

### Exact baseline

Baseline: `6d0454d84a044a830d1b898ad68f2e988d9a9129`. Initial measurements
were taken before editing exporter code. The final comparison also builds that
revision in an ignored directory using the **same final benchmark harness**:

```sh
# Use a fresh target/typescript-export-baseline directory.
mkdir -p target/typescript-export-baseline
git archive 6d0454d84a044a830d1b898ad68f2e988d9a9129 | tar -x -C target/typescript-export-baseline
mkdir -p target/typescript-export-baseline/specta-typescript/benches
cp specta-typescript/Cargo.toml target/typescript-export-baseline/specta-typescript/Cargo.toml
cp specta-typescript/benches/export.rs target/typescript-export-baseline/specta-typescript/benches/export.rs
cargo bench --manifest-path target/typescript-export-baseline/Cargo.toml --target-dir target/typescript-export-baseline-build -p specta-typescript --bench export > specta-typescript/benchmarks/baseline.txt
cargo bench -p specta-typescript --bench export > specta-typescript/benchmarks/pass1.txt
SPECTA_BENCH_MS=250 cargo bench -p specta-typescript --bench export -- prepared > specta-typescript/benchmarks/pass1-repeat.txt
SPECTA_BENCH_MS=250 cargo bench --manifest-path target/typescript-export-baseline/Cargo.toml --target-dir target/typescript-export-baseline-build -p specta-typescript --bench export -- prepared > specta-typescript/benchmarks/baseline-repeat.txt
```

The manifest copy only adds the benchmark target to the baseline; dependencies
are unchanged. Baseline source and its lockfile come from the archived revision.

Machine: Apple M5 Max, arm64, 128 GiB RAM; macOS 27.2.1 (26B131).
Toolchain: rustc 1.100.0 (`0ef1bc386`, LLVM 23.1.5), cargo 1.100.0
(`3acc79c0c`), default optimized bench profile. Measured 2026-11-19.

## Results

Raw results are in [`benchmarks/`](benchmarks/). `baseline.txt` and `pass1.txt`
contain all 80 cases at 150 ms/sample. The two `*-repeat.txt` files repeat the
20 rendering-only cases at 250 ms/sample, with optimized code measured before
the baseline to reverse the comparison order. All **80 full-run and 20 repeat
output byte lengths/checksums match** between revisions.

Rendering-only repeat (microseconds/export; larger speedup is better):

| Shape | Baseline | Pass 1 | Speedup |
| --- | ---: | ---: | ---: |
| chain / 32 | 238.54 | 39.06 | 6.11x |
| chain / 128 | 3,482.10 | 151.82 | 22.94x |
| chain / 512 | 53,427.18 | 609.38 | 87.67x |
| chain / 1024 | 218,497.40 | 1,247.97 | **175.08x** |
| enums / 1024 | 2,182.18 | 1,848.48 | 1.18x |
| records / 1024 | 1,187.34 | 1,148.65 | 1.03x |
| api / 512 | 883.62 | 863.62 | 1.02x |
| cycle / 1 | 1.797 | 1.774 | 1.01x |
| cycle / 128 | 15,945.52 | 16,963.03 | 0.94x |
| generic-api / 512 | 518.44 | 490.38 | 1.06x |

End-to-end full run, including Serde processing (milliseconds/export):

| Shape / layout | Baseline | Pass 1 | Speedup |
| --- | ---: | ---: | ---: |
| chain / 1024 / flat | 1,329.87 | 1,106.96 | **1.20x** |
| chain / 1024 / namespaces | 1,317.68 | 1,116.51 | 1.18x |
| chain / 1024 / jsdoc | 1,339.77 | 1,115.92 | 1.20x |
| enums / 1024 / flat | 4.096 | 3.664 | 1.12x |
| records / 1024 / flat | 703.27 | 689.84 | 1.02x |
| api / 512 / flat | 7.466 | 7.476 | 1.00x |
| cycle / 128 / flat | 50.810 | 50.726 | 1.00x |

The main result is a scaling change, not a universal 175x exporter speedup.
Rendering the acyclic chain is now approximately linear: increasing 32 to 1024
declarations multiplies runtime by about 32, versus about 916 before. The full
150 ms/sample run independently measured a 166.79x rendering-only improvement
at 1024 declarations. Serde processing still dominates the end-to-end chain
and record workloads.

**Uncertainty and tradeoffs:** this is a shared developer machine, without CPU
affinity/frequency control or statistical confidence intervals. Seven sample
means and an order-reversed repeat are not a controlled performance study.
Do not infer meaningful broad gains from the record, API, or generic controls.
Some control medians worsened by roughly 5–7%: the 128-cycle rendering-only case
was 1.5% slower in the full run and 6.4% slower in the repeat; the 512-response
generic namespace case was 6.9% slower end-to-end. Multi-node cycles do incur
an extra proof prepass before falling back, whereas direct self-cycles bypass it.
Those small differences remain unresolved; this pass does not claim zero
regressions on every shape. The large acyclic-chain scaling improvement is
consistent across sizes and runs.

## Validation

```sh
cargo test -p specta-tests --test test
cargo test -p specta-tests --test test -- --skip java::
cargo test -p specta-typescript
cargo clippy -p specta-typescript --all-targets
git diff --check
```

- Full integration run: 887 passed; 20 Java compilation tests failed because this
  machine has no Java runtime. No TypeScript, Serde, snapshot, or other tests
  failed. Skipping the Java module: **875 passed**, 32 filtered out.
- TypeScript crate: no unit tests; **18 doctests passed**, 4 existing ignored.
- Clippy succeeds, with no warnings in the changed exporter or benchmark code
  (existing dependency/workspace warnings may still appear).
- Four new regression tests cover a 128-node chain in both export orders,
  branching/shared tails with duplicate edges, a cyclic component next to a cached
  tail (including null and JSDoc), and cache isolation between batches and cloned
  declarations. The 36 existing recursion tests also pass, including generic
  argument-revealed edges, opaque failures, and divergent substitutions.

## Pass 2: reuse successful Serde validation visits

The phase probe confirmed that the remaining chain cost was validation, not
rewriting or TypeScript rendering. On pass 1, the 1024-node chain spent about
1.10 seconds validating and 2.3 ms cloning/rewriting its graph. The probe used
`Instant` immediately before `validate_for_mode`, immediately after it, and
before returning from `Format::map_types`; it printed the two elapsed durations
when `SPECTA_PROFILE` was set. The temporary instrumentation was removed before
the full comparisons. Raw probe output is in `benchmarks/pass2-profile-before.txt`
and `benchmarks/pass2-profile-visits.txt` (the latter measures visit reuse before
the field-key fast path).

```sh
SPECTA_PROFILE=1 SPECTA_BENCH_MS=1 cargo bench -p specta-typescript --bench export -- chain/1024/flat
```

Previously, each declaration and registered root started a new visited set.
The validator now separates a root's cycle guard from batch-local successful
proofs. Only after the **whole root succeeds** does it promote candidate visits;
an unfinished cycle is not a proof. Keys retain the reference's identity and
flatten directions. Ordinary references reset diagnostic paths to their named
declaration, so these successful proofs do not depend on the caller's path.
Every declaration body is still checked in the original iteration order.

Reuse is deliberately conservative. References with generic arguments, inline
bodies, recursive placeholders, or nonempty substitution environments keep the
original per-root validation. A root that encounters any such context publishes
no new proofs, even for its otherwise reusable visits: its bounded generic-key
fallback may have influenced the cycle guard. Existing proofs can still serve
context-free visits. The cache never survives `validate_for_mode`; individual
datatype and shallow validation have no completed-proof cache. Hash sets retain
exact keys rather than probabilistic hashes; a vector stages new proofs until
root completion. The original local visited set is cleared between roots and
retains its allocation.
Each substitution frame also records whether its scope chain has any bindings.
The reuse check reads that bit in constant time rather than rescanning a long
chain of empty frames; empty frames themselves remain intact, preserving lexical
substitution behavior.

A separate fast path avoids building two owned names and an occupied-key hash
table for each named field when both rename rules are absent and **all field
attributes are empty**. Those keys are identical in both directions and cannot
have aliases. Any field metadata or container rename rule keeps the full check.
Other field checks still run. This targets ordinary API and map-heavy records,
not just long synthetic chains.

### Pass 2 reproduction

The final harness adds `maps/32`, `maps/256`, and `maps/1024`: records containing
maps keyed by a shared renamed unit enum, integers, and strings, with shared user
records as values. All four layouts run for every shape: **92 cases** total.
The original 80 fixtures are unchanged. Both archived revisions use this same
final harness and benchmark manifest, with their own source and lockfile.

```sh
mkdir -p target/typescript-export-pass1
git archive 571e8d03 | tar -x -C target/typescript-export-pass1
# Reuse/create the original baseline archive as described above.
mkdir -p target/typescript-export-pass1/specta-typescript/benches
cp specta-typescript/Cargo.toml target/typescript-export-pass1/specta-typescript/Cargo.toml
cp specta-typescript/benches/export.rs target/typescript-export-pass1/specta-typescript/benches/export.rs
cp specta-typescript/benches/export.rs target/typescript-export-baseline/specta-typescript/benches/export.rs
cargo bench --manifest-path target/typescript-export-baseline/Cargo.toml --target-dir target/typescript-export-baseline-build -p specta-typescript --bench export > specta-typescript/benchmarks/pass2-baseline.txt
cargo bench --manifest-path target/typescript-export-pass1/Cargo.toml --target-dir target/typescript-export-pass1-build -p specta-typescript --bench export > specta-typescript/benchmarks/pass2-pass1.txt
cargo bench -p specta-typescript --bench export > specta-typescript/benchmarks/pass2.txt
```

The three full runs execute serially, baseline then pass 1 then pass 2, at the
default 150 ms/sample on the same machine/toolchain documented above. Fixture
construction and Serde preparation remain outside timed rendering-only cases.
Full-export cases include validation, rewrites, rendering, and output drop.

### Pass 2 results

All **92 full-run output byte lengths and checksums match** across the original
baseline, pass 1, and pass 2. Full-run medians (milliseconds/export):

| Shape / layout | Original baseline | Pass 1 | Pass 2 | Pass 1 → 2 speedup |
| --- | ---: | ---: | ---: | ---: |
| chain / 32 / flat | 1.434 | 1.232 | 0.239 | 5.16x |
| chain / 128 / flat | 22.090 | 17.718 | 1.012 | 17.50x |
| chain / 512 / flat | 372.817 | 290.534 | 3.881 | 74.86x |
| chain / 1024 / flat | 1,363.423 | 1,141.495 | 7.723 | **147.81x** |
| chain / 1024 / namespaces | 1,330.004 | 1,143.835 | 7.941 | 144.04x |
| chain / 1024 / jsdoc | 1,372.559 | 1,139.758 | 8.975 | 126.99x |
| records / 1024 / flat | 695.225 | 695.760 | 3.689 | **188.60x** |
| api / 512 / flat | 7.473 | 7.249 | 6.212 | 1.17x |
| maps / 1024 / flat | 13.353 | 13.242 | 11.654 | 1.14x |
| cycle / 128 / flat | 51.871 | 52.084 | 16.595 | 3.14x |
| enums / 1024 / flat | 4.081 | 3.518 | 3.624 | 0.97x |
| generic-api / 512 / flat | 2.536 | 4.043 | 2.423 | 1.67x |
| chain / 1024 / prepared-flat | 217.390 | 1.219 | 1.218 | 1.00x |

The chain's **end-to-end** runtime now scales approximately linearly: 32× more
declarations costs 32.3× more time. Combined with pass 1, the 1024-chain is
176.55× faster than the original baseline end-to-end, not just during rendering.
Linked records improve without involving alias collapse. Actual cyclic alias
graphs benefit from cheaper Serde validation, but their TypeScript collapse
algorithm is unchanged. The API and map gains are smaller and should not be
presented as universal hundred-fold improvements.

The full run's large apparent generic-API gain is **not** a reliable estimate:
its pass-1 control was unusually slow, including rendering-only code that this
pass does not change. The order-reversed repeats below are the better comparison.
The only full-run regression above 5% versus pass 1 was `enums/256/jsdoc` (+8.6%);
`enums/1024/flat` was 3.0% slower. These allocation-light controls need repeat
measurements, not an assumption that all changes help.

Order-reversed repeat, 250 ms/sample, measured **pass 2, then pass 1, then
original baseline**, with each revision's selected cases run serially:

| Flat end-to-end case | Original baseline (ms) | Pass 1 (ms) | Pass 2 (ms) | Pass 1 → 2 speedup |
| --- | ---: | ---: | ---: | ---: |
| chain / 1024 | 1,328.507 | 1,099.962 | 7.775 | **141.47x** |
| records / 1024 | 682.220 | 665.471 | 3.733 | **178.26x** |
| api / 512 | 7.047 | 7.083 | 6.257 | 1.13x |
| maps / 1024 | 13.248 | 13.484 | 11.726 | 1.15x |
| cycle / 128 | 51.269 | 51.278 | 16.919 | 3.03x |
| generic-api / 512 | 2.513 | 3.024 | 2.397 | 1.26x |
| enums / 1024 | 3.941 | 4.557 | 3.513 | 1.30x |

All **7 repeat outputs match** across the three revisions. The repeat confirms
the large scaling wins and the more modest API/map wins. Generic and enum
controls vary substantially between runs, including the unchanged pass-1 code;
do not interpret their reported speedup as an isolated pass-2 effect.

These commands reproduce both the end-to-end repeat files and a second set of
targeted control repeats. The latter includes the regressed enum/JSDoc case and
unchanged rendering-only paths; both sets use the same 250 ms sample duration.

```sh
python3 - <<'PY'
import os
import subprocess

for suffix, filters in [
    ("repeat", ["chain/1024/flat", "records/1024/flat", "api/512/flat",
                "maps/1024/flat", "cycle/128/flat", "enums/1024/flat"]),
    ("controls", ["enums/256/jsdoc", "generic-api/512/prepared-flat",
                  "maps/1024/prepared-flat", "chain/1024/prepared-flat",
                  "cycle/128/prepared-flat"]),
]:
    for revision in ("pass2", "pass1", "baseline"):
        args = [] if revision == "pass2" else [
            "--manifest-path", f"target/typescript-export-{revision}/Cargo.toml",
            "--target-dir", f"target/typescript-export-{revision}-build",
        ]
        path = f"specta-typescript/benchmarks/pass2-{revision}-{suffix}.txt"
        with open(path, "w") as output:
            for case in filters:
                subprocess.run(
                    ["cargo", "bench", *args, "-p", "specta-typescript",
                     "--bench", "export", "--", case],
                    env={**os.environ, "SPECTA_BENCH_MS": "250"},
                    stdout=output, check=True,
                )
PY
```

The `api/512/flat` substring also selects `generic-api/512/flat`, hence seven
end-to-end repeat cases rather than six. CPU affinity, frequency, and other
developer-machine activity remain uncontrolled; sample ranges in the raw files
are useful context, not confidence intervals. No extra dependencies were added.

Targeted controls (microseconds/export; all **5 control outputs match**):

| Case | Original baseline | Pass 1 | Pass 2 |
| --- | ---: | ---: | ---: |
| enums / 256 / jsdoc | 1,518.705 | 1,411.560 | 1,371.262 |
| generic-api / 512 / prepared-flat | 479.951 | 551.007 | 488.022 |
| maps / 1024 / prepared-flat | 3,350.498 | 3,359.418 | 3,326.765 |
| chain / 1024 / prepared-flat | 216,617.875 | 1,210.503 | 1,252.108 |
| cycle / 128 / prepared-flat | 15,893.450 | 16,128.401 | 15,920.828 |

The enum/JSDoc regression did not reproduce: pass 2 was 2.9% faster than pass 1
in the targeted repeat. Rendering-only chain time was 3.4% slower than pass 1
here but essentially identical in the full run (1,218.491 vs 1,219.340 µs).
Rendering code is unchanged in pass 2; these results do not establish zero
regressions, but they do not show a repeatable large regression either. The
cycle rendering repeat is now within 0.2% of the original baseline; pass 1's
earlier 6.4% rendering-only difference is not reproduced in this measurement.

### Pass 2 validation

```sh
cargo test -p specta-tests --test test -- --skip java::
cargo test -p specta-typescript -p specta-serde
cargo clippy -p specta-typescript -p specta-serde -p specta-tests --all-targets
cargo fmt --all -- --check
git diff --check
```

- Integration tests: **880 passed**, 32 Java tests filtered out (no Java runtime).
- Serde crate: **9 unit tests and 2 doctests passed**, 3 doctests ignored.
- TypeScript crate: **18 doctests passed**, 4 ignored; no unit tests.
- Five new tests cover shared tails/duplicate edges and cycles in both modes,
  batch isolation after graph mutation and an exact nested error-path snapshot,
  concrete generic roots in both orders, phase-dead versus live generic roots,
  and field/container rename checks outside the plain-field fast path. The same
  five tests also pass on archived pass 1, including the exact error snapshot.
- Clippy, formatting, and whitespace checks pass. Existing warnings in the
  integration-test crate remain; the new validation/benchmark/test code has no
  Clippy warnings.

The validator remains recursive. A 128-node debug shared-tail test exhausts the
default test-thread stack on both pass 1 and pass 2; the regression test uses an
explicit 8 MiB thread stack. This pass does not solve that pre-existing depth
limit. The release benchmark's 1024-node graphs succeed on the main thread.

### Remaining work

Generic-sensitive validation still repeats across roots. Extending reuse there
needs a stronger proof than sharing the existing resolved-reference keys:
bounded substitution fallback and attribute-embedded datatypes need care.
Serde rewrites and per-datatype format callbacks are other candidates.
Truly cyclic alias components still repeat the generic-aware collapse walk per
export; benchmark them before attempting SCC-level reuse of substituted bodies.
