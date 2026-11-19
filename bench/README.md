# Derive benchmark harness

Measures the compile-time cost of `#[derive(Type)]`. Not wired into CI and not
part of the repository workspace: every fixture crate (`bench/derive-perf`,
`bench/derive-inst`) and `bench/serde-shim` declares an empty `[workspace]`
table, so `cargo` at the repository root ignores them.

## Layout

```
bench/
├── gen.py        # derive-perf: many derives, almost no instantiations
├── gen_inst.py   # derive-inst: few generic types, many instantiations
├── measure.py    # measure one fixture
├── harness.py    # shared measurement code (used by measure.py and ablate.py)
├── ablate.py     # per-construct ablations
├── derive-perf/  # generated
├── derive-inst/  # generated
└── serde-shim/   # no-op `#[derive(SerdeShim)]`
```

## Setup

```sh
python3 bench/gen.py                              # derive-perf, SCALE=4 → 1248 derives
python3 bench/gen_inst.py                         # derive-inst, INST=200 → 1000 instantiations
python3 bench/measure.py <label> [iterations] [--fixture derive-perf|derive-inst]
python3 bench/ablate.py [--n 200] [--iters 5] [group ...]
```

Both generators are deterministic and write a `fixture.json` next to the crate
(`derives`, `instantiations`, `root_calls`) which `measure.py` prints and stores
in the result file.

`gen.py` samples the shapes the macro has to handle: plain and tuple structs,
newtypes, enums with all four variant shapes, generic structs/enums with
bounds, defaults and `skip_default_generic`, serde metadata (`rename_all`,
`tag`, `flatten`, `skip`, `with`, `transparent`, deprecated containers, docs on
everything) and `#[specta(type|inline|optional)]` overrides.

`gen_inst.py` is the other half of the picture. `derive-perf` has many derives
but almost no *instantiations* of generic types with concrete arguments, so
rustc never monomorphises `Type::definition` for a concrete `T`. It emits:

* four generic containers — `Wrapper<T>`, `Pair<T, U>`, `Choice<T>` (enum) and
  `Bounded<T: Clone + Send + Sync>`;
* `INST` concrete argument types `A0..An` (a `#[derive(Type, Clone, Copy)]`
  newtype each);
* `INST` `UsesN` structs, each mentioning five distinct generic applications
  (`Wrapper<Ai>`, `Pair<Ai, Ai+1>`, `Choice<A7i>`, `Bounded<Ai>`,
  `Vec<Wrapper<Ai>>`);
* `INST` calls to `<Wrapper<Ai> as Type>::definition(types)` and the same for
  `Pair`, the way a user's `Types::register` / `collect` roots do.

At the default `INST=200` that is 1000 generic instantiations and 400 roots.
Cost scales linearly with `INST` (see below); use `INST=1000` to model a crate
with a few thousand instantiations.

`serde-shim` is a no-op derive that only registers `attributes(serde)`, so the
fixtures can carry serde metadata without pulling `serde_derive` into the
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

The JSON result also records `monomorphization_collector_{root_collections,
graph_walk}`; they are not printed so the table stays comparable across rounds.

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

## Round 3: instantiation cost

Round 3 targets crates that use a generic `#[derive(Type)]` type at many
concrete arguments. Two fixtures, measured back to back on the same tree
(min of 9 runs). This first table is the opening change; the round as a whole,
including the later passes, is summarised at the end.

| fixture        | metric             | before  | after   |          |
| -------------- | ------------------ | ------- | ------- | -------- |
| `derive-perf`  | `type_check_crate` | 0.278 s | 0.252 s | −9.4 %   |
|                | `codegen_crate`    | 0.109 s | 0.064 s | −41 %    |
|                | `total`            | 1.382 s | 1.322 s | −4.3 %   |
|                | `cpu`              | 1.800 s | 1.512 s | −16 %    |
| `derive-inst`  | `type_check_crate` | 0.053 s | 0.043 s | −19 %    |
|                | mono graph walk    | 0.177 s | 0.104 s | −41 %    |
|                | `codegen_crate`    | 0.333 s | 0.157 s | −53 %    |
|                | `total`            | 1.186 s | 0.830 s | −30 %    |
|                | `cpu`              | 2.312 s | 1.374 s | −41 %    |

The change is one new hidden method, `Types::with_generic_substitution_fn`,
which takes `fn(&mut Types) -> DataType` instead of `impl FnOnce`. The generic
form monomorphises its whole body (including the `catch_unwind` machinery)
once per closure type, and the derive calls it once per field; a `fn` pointer
is a single type, so the body is compiled once. `LLVM` lines for `derive-inst`
drop from 700,362 to 463,936; the `with_generic_substitution` pile shrinks from
34 % to a handful of copies. Runtime registration is unchanged (within noise):
20.3 ms vs 20.5 ms for 20k derived `definition` calls.

`inst-before`/`inst-final` and `round3-before`/`round3-final` in
`bench/results/` are the raw runs. Instantiations cost, at 200 (`INST=200`,
1000 instantiations):

* `type_check_crate` 0.053 s → 0.053 ms/instantiation
* mono graph walk 0.177 s → 0.177 ms/instantiation
* `codegen_crate` 0.333 s → 0.333 ms/instantiation

Scaling is linear: `INST=50/100/400` gave `codegen_crate` 0.066/0.132/0.582 s
and `type_check_crate` 0.016/0.028/0.095 s.

### Where the time goes (`-Zself-profile`, `derive-inst`, before the fix)

Only ~2.5 % of profiled time is trait selection (`typeck_root` 37 ms,
`evaluate_root_goal_for_proof_tree_raw` 37 ms, `codegen_select_candidate` 13 ms,
`define`-related `try_normalize` 13 ms, `resolve_instance_raw` 7 ms). More than
80 % is LLVM: `LLVM_module_codegen_emit_obj` 1.21 s, `LLVM_passes` 0.56 s,
`codegen_module` 0.32 s, `finish_ongoing_codegen` 0.32 s. Mono collection
(`items_of_instance` 65 ms self over 29,814 instances, `symbol_name` 39 ms over
27,051) is the next block. HIR type-checking and trait resolution are *not* the
bottleneck; per-instantiation machine code is.

### Per-construct costs (`ablate.py`, N=200)

Each variant repeats one construct 200 times; `Δ` is against the group's
baseline, in ms. `field-type` and `shape` change type-check, `generic` changes
codegen. Re-measured at the end of the round, so the absolute values also
include the earlier passes' wins. Full JSON in `bench/results/ablation.json`.

| group       | construct                  | bytes    | typeck   | codegen  |
| ----------- | -------------------------- | -------- | -------- | -------- |
| field-type  | empty struct (baseline)    | 92,711   | 0.006    | 0.002    |
|             | 4× `u64`                   | 183,311  | +7       | +3       |
|             | 4× `Vec<String>`           | 197,317  | +8       | +3       |
|             | 4× `Shared`                | 188,513  | +7       | +3       |
|             | 4× `String`                | 188,111  | +8       | +3       |
|             | 4× `Option<Vec<u64>>`      | 242,129  | +10      | +4       |
| shape       | unit struct (baseline)     | 89,499   | 0.007    | 0.002    |
|             | named struct, 1 field      | 107,705  | +1       | +1       |
|             | named struct, 3 fields     | 155,705  | +5       | +2       |
|             | newtype                    | 106,705  | +1       | +1       |
|             | tuple struct, 3 fields     | 139,505  | +4       | +2       |
|             | enum, 4 unit variants      | 177,713  | +10      | +8       |
|             | enum, 4 newtype variants   | 267,719  | +17      | +11      |
|             | enum, 4 tuple variants     | 407,715  | +29      | +17      |
|             | enum, 4 named variants     | 498,915  | +33      | +17      |
| metadata    | none (baseline)            | 107,705  | 0.008    | 0.003    |
|             | `serde(rename_all)`        | 205,129  | +6       | +2       |
|             | `serde(rename)` on field   | 168,333  | +4       | +2       |
|             | `serde(default)` field     | 141,335  | +2       | +0       |
|             | `serde(flatten)` field     | 149,535  | +2       | +1       |
|             | `serde(skip)` field        | 144,329  | +2       | +0       |
|             | `specta(optional)` field   | 129,727  | +1       | +0       |
|             | docs everywhere            | 152,905  | +1       | +0       |
|             | deprecated everywhere      | 166,517  | +6       | +3       |
|             | `serde(tag)` container     | 173,335  | +4       | +1       |
| generic     | one bare `T` field         | 373,103  | 0.018    | 0.007    |
|             | `T` + `Vec<T>`             | 374,035  | +0       | +7       |
|             | two params                 | 378,046  | +0       | +8       |
|             | `Option<T>` unbounded      | 375,317  | +0       | +3       |
|             | `Option<T>` bounded        | 375,384  | +0       | +11      |
|             | `Vec<T>`                   | 372,709  | +0       | +6       |
|             | hand-written cheap list    | 375,596  | +0       | +2       |
|             | `Option<Vec<T>>`           | 375,617  | +1       | +9       |
| field-arg   | `.field(name, dt)`         | 185,420  | 0.016    | 0.006    |
|             | `.field(name, Field::new)` | 205,620  | +2       | +0       |
|             | plain `Field` argument     | 211,620  | +0       | +0       |

Readings:

* Field and struct shapes are cheap and flat (~7 ms per 800 fields, i.e.
  9 µs per field). Enums are not: 200 enums with four variants cost 10–33 ms
  type-check against a unit struct, so a unit variant costs ~12 µs and a variant
  with three named fields ~41 µs.
* Container serde metadata and `deprecated` are the expensive attributes
  (~30 µs per container for `rename_all`, +6 ms/200); `specta(optional)` is
  cheaper than `serde(default)`.
* Field metadata is now close to free: a `rename`, a `skip` or a `flatten` is
  +2 ms of type-check per 200 fields, and docs or `specta(optional)` +1, against
  +3…+5 before pass 4. The bytes column shows the same: `serde(rename)` on a
  field is 168,333 bytes now, 215,226 when the metadata was a statement.
* A generic `Vec<T>` field costs ~0.030 ms of codegen per instantiation, while a
  hand-written list impl with the same `DataType` costs ~0.010 ms. The delta is
  the named-type registration machinery in specta's `impl_ndt` passthrough
  impl, not the derive.
* Bounded generic parameters are more expensive than unbounded ones (+11 ms vs
  +3 ms for 200 `Option<T>` instantiations) because the derive cannot resolve a
  bounded parameter through a placeholder — the field type's own declaration may
  require the bound — so those declarations go through the generic-substitution
  scope instead (see pass 2 below).
* `impl Into<Field>` is **not** a cost. 1000 fields cost 16 ms type-check with
  `.field(name, dt)`, 16 ms with a plain `Field` argument, and 18 ms with
  `Field::new(dt)` at the call site (the extra call is slightly worse; run-to-run
  noise is ±3 ms). The pass-7 relaxation is a token win (~20 bytes/field) and
  free on the compiler.

### Generic container shape (a/b/c), 200 instantiations

Hand-written versions of the same `W<T> { a: T, b: Vec<T> }` (measured before
pass 2, so `a-current` is the shape the derive used to emit). The crates that
produced this table called `Types::define_split`, which pass 2 removed, so the
group is no longer part of `ablate.py`; the table is the record of the
measurement that motivated the split.

| variant      | what it does                                             | codegen |
| ------------ | -------------------------------------------------------- | ------- |
| `a-current`  | one generic `build<T>` with declaration and concrete arms | 43 ms   |
| `b-split`    | declaration build is a non-generic nested fn              | 40 ms   |
| `c-hoisted`  | also the `build_ndt` metadata closure is non-generic      | 34 ms   |
| `none`       | same `A` impls, no instantiations                         | 3 ms    |

Splitting only the declaration *build* is within noise (0–3 ms here; an earlier
run read them as equal). Hoisting the whole declaration path — build *and*
metadata closure — out of the generic body is worth ~0.045 ms per instantiation
(~16 % of container codegen). The remaining per-instantiation cost is dominated
by `Types::define_param::<T>` (88 LLVM lines each).

### Pass 2: what a generic container emits now

The declaration half of a field resolution never mentions the container's type
parameters (it resolves placeholder types), so the derive no longer puts it in
the generic body:

```rust
// Unbounded parameters: the declaration *is* the concrete build resolved at the
// placeholders, so there is only one body.
fn __concrete<T, U>(types: &mut __Types) -> __D::DataType where T: __T, U: __T {
    // concrete resolutions only
}
…
types.define_instantiation_params(
    …, &[InstantiationParam::Resolved { … }, …], &[type_name::<T>(), …],
    __concrete::<T, U>, __concrete::<__P0, __P1>,
)

// Bounded, const or opaque: the declaration keeps a generic body of its own,
// parameterised over the placeholder types where it can be.
fn __declaration<__P0: __T>(types: &mut __Types) -> __D::DataType {
    // placeholder resolutions only
}
…
types.define_instantiation_params(…, __concrete::<T>, __declaration::<__P0>)
```

Two details make that pay off:

* A placeholder's own definition *is* its `Generic`, so for a container whose
  parameters are all unbounded the declaration is exactly `__concrete` resolved
  at the placeholders — no second body at all. Bounded parameters (and const
  parameters, opaque `macro!` field types and container attributes that resolve
  a user type) keep a separate `__declaration`, and are the only containers whose
  declaration build is still instantiated per concrete argument.
* `__declaration` is parameterised over the placeholder *types*, not the
  container's parameters. That keeps it a generic item, so rustc only
  monomorphises it once the container is actually used: a crate that declares
  generic types but never instantiates them compiles no declaration body at all.
  A non-generic nested fn is compiled eagerly, which cost 0.063 → 0.110 s of
  `codegen_crate` on `derive-perf` (no `collect`, so no instantiations).

`Types::define_instantiation_params` resolves the parameter table and reads the
substitution cursor itself, so a `definition` body no longer builds the
`[(Generic, DataType); N]` array, its drop glue or the identity array; it passes
one `&[InstantiationParam]` of `&'static` data instead. Measured back to back on
this tree (min of 11); re-emitting the old shape from the same tree reproduced the
"before" column (0.117 s codegen on `derive-inst`, 0.238 s type check on
`derive-perf`), so the deltas are the change and not drift:

| fixture       | metric             | before  | after   |         |
| ------------- | ------------------ | ------- | ------- | ------- |
| `derive-inst` | `codegen_crate`    | 0.119 s | 0.087 s | −27 %   |
|               | mono graph walk    | 0.076 s | 0.058 s | −24 %   |
|               | `total`            | 0.703 s | 0.600 s | −15 %   |
|               | `cpu`              | 1.106 s | 0.921 s | −17 %   |
|               | LLVM lines / items | 302,249 / 14,092 | 254,759 / 9,482 | −16 % / −33 % |
| `derive-perf` | `type_check_crate` | 0.241 s | 0.230 s | −4.6 %  |
|               | `codegen_crate`    | 0.063 s | 0.064 s | —       |
|               | `total`            | 1.283 s | 1.237 s | −3.6 %  |
|               | `cpu`              | 1.467 s | 1.424 s | −2.9 %  |
|               | LLVM lines / items | 193,033 / 5,046 | 192,864 / 5,046 | — |

Repeated 11-run measurements of this tree read 0.085–0.087 s and 1.215–1.237 s,
so treat the last digit as noise (±3 %).

`derive-perf` cannot re-emit the old shape through the derive (it instantiates
nothing), so its LLVM row is the previous expansion compiled as a crate; the item
count matching confirms the comparison. It also shows why the declaration has to
stay lazily monomorphised: this fixture gains nothing from the split, and an
eagerly compiled declaration body cost it 0.047 s of codegen.

Registration cost is unchanged (659 vs 652 ns per `definition` call into a fresh
`Types`, 20k calls, release): the parameter `Vec` replaces the one
`NamedDataType::init` used to allocate by cloning the caller's array.

What is left per `derive-inst` instantiation is the concrete build itself
(`Choice` 202 LLVM lines, `Pair` 100, `Wrapper` 54, `Bounded` 60), plus
`Bounded`'s declaration (118, bounded so per argument), the `definition` body
(~63–78, dominated by the parameter table and the call) and the container's own
field datatypes. Nothing in those is redundant short of the datatype the user
asked to build. The next lever is the same parameter table in `impl_ndt`'s
`definition` for the built-in containers (`Vec<T>`, `Option<T>`, …), which still
build the array themselves.

### Pass 3: the parameter table everywhere, and no temporaries in `definition`

Pass 3 applies pass 2's treatment to the built-in containers and removes the
last per-instantiation work that was not the datatype itself. Everything is
measured against this tree at the start of the pass, back to back in the same
session (min of 11):

| fixture       | metric             | before  | after   |         |
| ------------- | ------------------ | ------- | ------- | ------- |
| `derive-inst` | `expand_crate`     | 0.036 s | 0.035 s | —       |
|               | `type_check_crate` | 0.044 s | 0.036 s | −18 %   |
|               | `codegen_crate`    | 0.089 s | 0.057 s | −36 %   |
|               | `total`            | 0.609 s | 0.485 s | −20 %   |
|               | `expanded_bytes`   | 484,922 | 434,836 | −10 %   |
|               | LLVM lines / items | 254,759 / 9,482 | 169,242 / 5,866 | −34 % / −38 % |
| `derive-perf` | `expand_crate`     | 0.176 s | 0.174 s | —       |
|               | `type_check_crate` | 0.229 s | 0.204 s | −11 %   |
|               | `codegen_crate`    | 0.063 s | 0.051 s | −19 %   |
|               | `total`            | 1.233 s | 1.103 s | −11 %   |
|               | `expanded_bytes`   | 2,569,088 | 2,343,925 | −8.8 % |
|               | LLVM lines / items | 192,864 / 5,046 | 181,829 / 3,096 | −5.7 % / −39 % |

Repeated 11-run measurements read 0.057–0.059 s (`derive-inst` codegen),
0.172–0.174 s and 0.202–0.204 s (`derive-perf` expand/type check).
`p3-before-inst`/`p3-before-perf` and the `p3-after-*` files in `bench/results/`
are the raw runs.

Per-call registration work is unchanged: the empty substitution scope and its
`catch_unwind` were already paid once per non-generic `definition` call, they are
now paid inside `Types::define_named` instead of in the generated body. The
parameter `Vec` likewise replaces the one `NamedDataType::init` used to allocate
by cloning the caller's array.

Four changes, each purely a reduction of what the generated code contains:

* `impl_ndt!` passes a `&[InstantiationParam]` to
  `Types::define_instantiation_params` instead of building the
  `[(Generic, DataType); N]` array and calling `define_instantiation`. The table
  is `&'static` data (a name, a `fn` pointer and a tag) and is const-promoted, so
  a `Vec<X>` instantiation builds nothing. An inline implementation whose
  `where` clause keeps a parameter opaque (`Cow`) uses a new
  `InstantiationParam::Generic` variant that reports the declaration's own
  `Generic` without resolving a slot.
* The two datatype builders are the function item `Types::define::<..>` instead of
  a closure around it. The method is monomorphised either way — it is what
  resolves the datatype — but the closure added a body *and* a `call_once` shim
  per instantiation. `Vec<X>::definition` drops from 5 items to 1.
* `generic declarations` and `deprecated` are borrowed (`&'static [GenericDefinition]`,
  `Option<&'static Deprecated>`). Passing them by value made every
  `definition` body materialise a `Cow` and an `Option<Deprecated>` temporary,
  with a drop flag, two drop-glue calls and a cleanup path each. Since a parameter
  default can name another parameter (`U = T`), that list is built at runtime for
  those types: they keep a separate owned entry point, which is the only path
  that still pays for the temporaries. `Deprecated::with_note`/`with_since_note`
  became `const fn` so the metadata can live in a `static`.
* A container with no type parameters at all enters the empty
  generic-substitution scope it needs inside `Types::define_named` instead of in
  the derive, which was emitting a `with_generic_substitution(&[], ..)` closure
  around the whole body — a second copy of the body plus two items per derived
  type.

Together they take `derive-inst`'s `definition` bodies from ~55–93 LLVM lines to
17–20: two static loads, `type_name::<T>()` and one call. What remains per
instantiation is exactly the user's datatype (`Choice` 202, `Pair` 100,
`Wrapper` 54, `Bounded` 60 + 118 for its per-argument declaration), the
parameter/identity tables (promoted constants) and the `define*` shims those
tables point at.

Two shapes were priced and rejected:

* A `&'static InstantiationSpec` struct holding every non-generic argument, so the
  call has 5 arguments instead of 13. It reaches 166,836 LLVM lines and 0.054 s of
  `derive-inst` codegen, but the struct literal grows the macro's output by 3.4 %
  (+79 KB, mostly the field names of one `static` per generic container) and
  costs `derive-perf` ~4 ms of type check, which this pass is not allowed to
  trade away. The 13-argument call is a shared non-generic body either way; only
  the call site differs.
* A generic `fn __build<T: Type>(types) -> DataType` as the builder instead of a
  closure (or the `Types::define::<T>` function item). It would avoid one
  monomorphisation, but `@builders` does not receive the implementation's bounds,
  so `Box<T>`'s `T: ?Sized` cannot be reproduced; the function item gets the same
  win without the plumbing.

### Pass 4: field metadata is an expression, not a statement

A field with metadata used to be a `let mut` binding that was mutated statement
by statement and then read:

```rust
let mut field3 = __D::Field::new(<Option<bool> as __T>::definition(types));
field3.attributes.insert("specta:nullable", true);
field3
```

That is the common case, not the rare one: `derive-perf` has 1,601 such fields,
because every field declared as an `Option` records the `specta:nullable` flag
(1,081 of the 2,121 field attributes in that crate are that one key). At `-O0`
the binding also owns a drop flag and a cleanup edge in the enclosing `build` —
`Field` is a 208-byte value owning a `HashMap`, a `Cow` and the `DataType` — and
each statement carried 3–9 tokens of scaffolding (the binding reference,
`.attributes.insert` and the terminator) around the metadata itself.

`datatype::Field` gained five `#[doc(hidden)]` consuming setters —
`optional()`, `nullable()`, `docs(&'static str)`, `deprecated(Option<Deprecated>)`
and `attribute(key, value)` — so a field is one expression:

```rust
__D::Field::new(<Option<bool> as __T>::definition(types)).nullable()
```

`nullable` is separate from `attribute` because it is the one flag a derive sets
on every `Option` field; it is now the only place the key is spelled. The serde
attributes chain through `attribute(..)` with the same value expressions as
before (`"aField".to_owned()`, `Vec::from([..])`, and the resolved `DataType` of
a `from`/`try_from`/`into` conversion), so the lowerer now builds key/value
entries and emits them either as `#target.insert(..)` statements — containers and
variants, whose attributes are still written through a place expression on a
value mutated in place — or as chained calls. Same keys, same values, same order
of evaluation, same datatypes: the 882 snapshot tests are unchanged.

| fixture       | metric             | before    | after     |         |
| ------------- | ------------------ | --------- | --------- | ------- |
| `derive-perf` | `expanded_tokens`  | 1,423,359 | 1,332,099 | −6.4 %  |
|               | `expanded_bytes`   | 2,343,925 | 2,217,712 | −5.4 %  |
|               | `expand_crate`     | 0.172 s   | 0.168 s   | −2.3 %  |
|               | `type_check_crate` | 0.207 s   | 0.194 s   | −6.3 %  |
|               | `codegen_crate`    | 0.054 s   | 0.052 s   | −3.7 %  |
|               | `total`            | 1.126 s   | 1.095 s   | −2.8 %  |
| `derive-inst` | `expanded_tokens`  | 296,092   | 295,912   | —       |
|               | `expand_crate`     | 0.035 s   | 0.035 s   | —       |
|               | `type_check_crate` | 0.037 s   | 0.037 s   | —       |
|               | `codegen_crate`    | 0.058 s   | 0.058 s   | —       |

The `derive-inst` fixture has almost no `Option` fields, so it only gets the
smaller emission (180 bytes of expansion) and is otherwise unchanged.

By construction the pass removes ~22.5 k emitted tokens, 4.7 % of the derive's
output: 1,601 × 6 for the bindings and their use sites, and 2,121 × 3–9 for the
statements. At the calibration that is ~7 ms of `expand_crate`; measured back to
back it was 4 ms, inside the intercept's noise. The rest of the win is in
codegen: a value moved into a call is the callee's to drop, so the enclosing
`build` needs no drop flag and no cleanup edge for the field; the chain's own
temporaries are what is left (each setter takes and returns the `Field`, so the
moves are still there). The pair above was measured back to back in one session;
a later re-run of the final tree in a quieter session read 0.161/0.189/0.050 s,
so compare the pair rather than the absolute values across sessions.

Two shapes were priced and not taken:

* Chaining variant and container metadata too. Variants have 400 `let mut`
  bindings and 600 attribute statements left in `derive-perf`, containers 360 and
  480, all in the same shape. That needs four more hidden setters on `Variant`
  (`skip`, `docs`, `deprecated`, `attribute`) and a `variants(..)` setter on
  `Enum`, and is worth ~8 k tokens (<2 % of the emitted tokens, ~4 ms of
  expansion at the calibration): the variant values are collected into one array
  either way, so the binding only holds the metadata statements.
* Not passing `define_named`'s sentinel. The sentinel *is*
  `concat!(module_path!(), "::", stringify!(Name))` and `NamedId::Static`
  compares it as one `&'static str`. Dropping the argument would make the
  identity a `(module_path, name)` pair — stable, but no longer a `&'static str`,
  and the recursive-inline fallback would then need a `format!` for the path it
  reports. It is worth only ~16 tokens per derive (4.2 % of the macro's output,
  ~6 ms of `expand_crate` at the calibration) and is rejected on the identity
  change. The inverse — keep the sentinel and recover `name`/`module_path` by
  splitting it — saves 4 of those tokens, adds a runtime scan to every
  registration and needs a special case for raw identifiers (`stringify!(r#type)`
  is not the exported name), so it was rejected too.

### Notes, noise, and wrong hypotheses

* Trait resolution is not the lever at this scale. It is ~2.5 % of the
  self-profile and moves by <2 ms per 1000 fields in the field-arg ablation.
* The "declaration arm duplication is most of the cost" hypothesis is wrong:
  see b-split above.
* `borrow_check_crate` never appears in `-Ztime-passes` on rustc 1.102
  nightly; `measure.py` records it only if present.
* `finish_ongoing_codegen` is the noisiest pass (±20 %); trust `cpu` and
  `codegen_crate` for codegen changes.
* `Types::define`/`define_root`/`define_param` are now two instructions each
  (`type_name` plus an indirect call). Every remaining call site passes them as
  the `fn` pointer argument of `with_generic_substitution_fn` or as an
  instantiation builder, so replacing them with a `&'static str` + `fn` pair
  would just move the same shim into the caller.
* The two largest remaining per-instantiation items are the enum body
  (`Choice`, 202 lines) and a five-field struct body (`Uses`, 114). Both are the
  datatype the user asked for: at `-O0` each `Variant` is a 160-byte value moved
  through the builder chain with a drop flag, and no derive-side rewrite removes
  those moves without changing `specta::datatype`'s builder shapes.
* `#[specta(inline)]` fields still emit `datatype::inline(types, |types| ..)`,
  and that helper is generic over the closure, so each inline field
  monomorphises its body (30 LLVM lines) plus the closure. The 80 inline fields
  in `derive-perf` cost ~2.4k lines that way. Collapsing it needs
  `resolve_ty_expr` to also report when the resolved expression is a bare
  type-definition path, so the call can pass that function item to a
  non-generic `inline`; it is not on the instantiation path, so it was left.

### Round 3 summary

Round 3 starts at `round3-before.json`/`inst-before.json` and ends at
`round3-final.json`/`inst-final.json`. Both fixtures, min of 9–11 runs (the last
measurements ran in a quieter session than the round-start ones, so the deltas
are good to a few percent):

| fixture       | metric             | round start | round end |          |
| ------------- | ------------------ | ----------- | --------- | -------- |
| `derive-perf` | `expanded_tokens`  | 1,554,770   | 1,332,099 | −14 %    |
|               | `expand_crate`     | 0.180 s     | 0.161 s   | −11 %    |
|               | `type_check_crate` | 0.278 s     | 0.189 s   | −32 %    |
|               | `codegen_crate`    | 0.109 s     | 0.050 s   | −54 %    |
|               | `total`            | 1.382 s     | 1.041 s   | −25 %    |
|               | `cpu`              | 1.800 s     | 1.194 s   | −34 %    |
|               | mono items         | 10,511      | 3,084     | −71 %    |
| `derive-inst` | `expanded_tokens`  | 314,797     | 295,912   | −6.0 %   |
|               | `expand_crate`     | 0.037 s     | 0.034 s   | −8.1 %   |
|               | `type_check_crate` | 0.053 s     | 0.036 s   | −32 %    |
|               | `codegen_crate`    | 0.333 s     | 0.056 s   | −83 %    |
|               | `total`            | 1.186 s     | 0.474 s   | −60 %    |
|               | `cpu`              | 2.312 s     | 0.684 s   | −70 %    |
|               | mono items         | 29,814      | 5,850     | −80 %    |

The round is one idea applied in four places: *a `definition` body should contain
the user's datatype and nothing else.* A named type has two artifacts — the
canonical declaration, built from `Generic` placeholders and shared by every
instantiation, and the concrete build, which is the datatype itself. Before the
round the derive emitted the scaffolding for both inside `fn definition`, so
every concrete argument monomorphised a private copy of the bookkeeping, and
every derive emitted closures around its own body. Round 3 moved everything that
only depends on the *shape* of an implementation into a non-generic entry point
in `specta`, which the derive calls with `&'static` tables and `fn` pointers: the
instantiation parameter table and its identity list, the substitution cursor and
the empty scope a non-generic container needs, the declaration's
`GenericDefinition` list, the deprecation metadata (a `static`), and the metadata
builder itself. What is per-instantiation is now the datatype the user asked
for, one concrete build and one call; what is per-derive is one nested `build`
and one call.

That is why the two fixtures move differently. `derive-inst` (1000
instantiations of four containers) loses 80 % of its monomorphised items and 83 %
of its codegen because those per-argument copies were the cost. `derive-perf`
(1248 derives, no instantiations) loses 71 % of its items and 54 % of its codegen
because every derive stopped wrapping its body in closures, and it gains from
the last pass, which turned a field's metadata from a `let mut` plus statements
into one expression. Expansion is now linear in the tokens the macro emits rather
than in the number of instantiations: `expand_crate ≈ 300 ns × emitted tokens +
35 ms` (the calibration pass 12 established), and the derive's output for
`derive-perf` is ~457 k tokens (366/derive) after this pass, down from 479 k
(384/derive).

What is left, and why it was not taken:

* `derive-perf`'s `codegen_crate` is 0.050 s and almost all of it is the
  datatype construction itself: 168 k of the crate's 183 k LLVM lines are the
  1248 `build` bodies, ≈23 lines per field, i.e. one `Field` (208 bytes)
  constructed, one move into the `Fields` vector and one `Type::definition` call.
  The 0.048 s target for this pass needed a derive-side change, and there is not
  one left at that level; the next lever is a smaller `Field` (a `HashMap`, a
  `Cow`, the optional deprecation record and the `DataType`) or cheaper builder
  shapes in `specta::datatype`, both of which are public shape changes.
* Variant and container metadata still use the statement form (400 + 360
  bindings) — priced at <1 % of the emitted tokens in pass 4.
* `#[specta(inline)]`'s per-closure `inline` monomorphisation (~2.4 k LLVM lines)
  and the per-field builder temporaries' cleanup edges are the remaining
  scaffolding.
* `define_named`'s sentinel is the last per-derive token the identity model
  refuses to give up (see pass 4).
