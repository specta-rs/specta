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

## Next measurements

Serde graph processing is a separate, larger end-to-end cost on long reference
chains: compare `flat` against `prepared-flat`, including the record control
where alias collapse does not run. `specta-serde/src/validate.rs::validate_for_mode`
starts a fresh checked-reference set for every declaration and registered root.
Profile this before changing it; validation depends on generic environments,
flattening directions, and error paths, so a global visited set is not automatically
correct. Serde rewrites and per-datatype format callbacks are other candidates.
Truly cyclic alias components still repeat the generic-aware collapse walk per
export; benchmark them before attempting SCC-level reuse of substituted bodies.
