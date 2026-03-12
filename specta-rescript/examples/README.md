# Examples

Each example focuses on a different aspect of ReScript code generation. Pre-generated `.res` files live in `generated/` for quick inspection without running Cargo.

## Usage

**Generate `.res` files** by running any example:

```bash
cargo run -p specta-rescript --example simple_usage
cargo run -p specta-rescript --example basic_types
# etc.
```

**Type-check the generated ReScript** using the ReScript compiler:

```bash
cd examples/generated
npm install        # installs the rescript compiler
npm run check      # runs: rescript build
```

The `lib/` directory produced by `rescript build` is gitignored.

---

## simple_usage

**Example:** [`simple_usage.rs`](simple_usage.rs)
**Generated:** [`generated/SimpleUsage.res`](generated/SimpleUsage.res), [`generated/SimpleUsageCustomHeader.res`](generated/SimpleUsageCustomHeader.res)

| Input (Rust) | Output (ReScript) | Description |
|---|---|---|
| `struct User { id: u32, name: String, ... }` | `type user = { id: int, name: string, ... }` | Named struct |
| `enum UserRole { Guest, Moderator, Admin }` | `type userRole = [ #Guest \| #Moderator \| #Admin ]` | All-unit enum -> polymorphic variants |
| `enum ApiResponse<T> { Ok(T), Err(String) }` | `type apiResponse<'t> = result<'t, string>` | Result-shaped enum -> built-in `result` |
| `struct CreateUserRequest { role: UserRole, ... }` | `type createUserRequest = { role: userRole, ... }` | Struct referencing another named type |

---

## basic_types

**Example:** [`basic_types.rs`](basic_types.rs)
**Generated:** [`generated/BasicTypes.res`](generated/BasicTypes.res)

| Input (Rust) | Output (ReScript) | Description |
|---|---|---|
| `i8`, `i16`, `i32`, `i64`, `u8`..`u64`, `isize`, `usize` | `int` | All integer types |
| `f32`, `f64` | `float` | Float types |
| `bool` | `bool` | Boolean |
| `char`, `String` | `string` | String types |
| `Option<T>` | `option<t>` | Optional value |
| `Vec<T>`, `[T; N]` | `array<t>` | List / fixed-length array |
| `Vec<Vec<T>>` | `array<array<t>>` | Nested array |
| `HashMap<String, V>` | `dict<v>` | String-keyed map |
| `(T1, T2, T3)` | `(t1, t2, t3)` | Tuple |
| Struct referencing another struct | Named type reference | Nested structs |

---

## variants

**Example:** [`variants.rs`](variants.rs)
**Generated:** [`generated/Variants.res`](generated/Variants.res)

| Input (Rust) | Output (ReScript) | Description |
|---|---|---|
| `enum Status { A, B, C }` | `type status = [ #A \| #B \| #C ]` | All-unit enum -> polymorphic variants |
| `Notification(String)` | `\| Notification(string)` | Single unnamed-field variant |
| `TwoD(f64, f64)` | `\| TwoD(float, float)` | Multi-field tuple variant |
| `Line { x1: f64, y1: f64, ... }` | auxiliary record type + `\| Line(shapeLineFields)` | Named-field variant -> auxiliary record |
| Mixed unit + data variants | regular variants throughout | Mixed enum |
| `MoveTo(Point)` | `\| MoveTo(point)` | Variant referencing a named type |

---

## result_types

**Example:** [`result_types.rs`](result_types.rs)
**Generated:** [`generated/ResultTypes.res`](generated/ResultTypes.res)

| Input (Rust) | Output (ReScript) | Description |
|---|---|---|
| `enum Foo { Ok(T), Err(E) }` | `result<t, e>` | Any `Ok`/`Err` enum -> built-in `result` |
| `Option<T>` | `option<t>` | Built-in `option` |
| `field: MyResult<A, B>` | `field: result<a, b>` | `result` as a field type |
| `Vec<MyResult<A, B>>` | `array<result<a, b>>` | `result` inside an array |
| Generic `FetchResult<T>` | `type fetchResult<'t> = ...` | Generic struct with result field |

---

## generics

**Example:** [`generics.rs`](generics.rs)
**Generated:** [`generated/Generics.res`](generated/Generics.res)

| Input (Rust) | Output (ReScript) | Description |
|---|---|---|
| `struct Wrapper<T>` | `type wrapper<'t>` | Single generic param |
| `struct Pair<A, B>` | `type pair<'a, 'b>` | Multiple generic params |
| Generic param `T` used in a field | `'t` | Generic type expression |
| `CachedValue<T>` wrapping `Wrapper<T>` | `type cachedValue<'t> = { inner: wrapper<'t>, ... }` | Nested generic reference |
| `Container<T> { Empty, Single(T), Multiple(Vec<T>) }` | `type container<'t> = \| Empty \| Single('t) \| Multiple(array<'t>)` | Generic enum |

---

## comments_example

**Example:** [`comments_example.rs`](comments_example.rs)
**Generated:** [`generated/CommentsExample.res`](generated/CommentsExample.res)

| Input (Rust) | Output (ReScript) | Description |
|---|---|---|
| `/// Type doc comment` | `// Type doc comment` | Type-level doc comment |
| Multi-line `///` on a type | Multiple `//` lines before the type | Multi-line type doc comment |
| `#[deprecated = "use Foo instead"]` on a type | `// @deprecated use Foo instead` | Type-level deprecated marker |

> **Note:** Field-level and variant-level doc comments and `#[deprecated]` markers are not currently emitted.
