# Migrating to Specta 2.0

This guide moves an application from a Specta 2.0 release candidate to 2.0.0. Most of the breaking changes landed in rc.23, rc.24, and rc.25, and each "Since" column says which ones your release candidate already has. Upgrade `specta` and its exporter crates together, because every exporter pins an exact `specta` version.

## Feature flags

Each third-party feature now carries the supported version in its name: the major for 1.x and later (`uuid1`, `tokio1`), and the major and minor for 0.x (`chrono04`, `glam034`). The [feature list](https://docs.rs/specta/latest/specta/#feature-flags) names every one.

| Before | 2.0 | Since |
| --- | --- | --- |
| `uuid`, `chrono`, and the other unversioned names | the versioned names above | after rc.28 |
| `export` | `collect` | rc.23 |
| `serde` | removed; Serde behavior comes from passing `specta_serde::Format` to an exporter | rc.23 |
| `interop` | removed | rc.23 |
| no `std` feature | `std`, on by default; enable it if you turn default features off | rc.23 |
| `function` | a no-op kept for Tauri 2.x | after rc.28 |

Some features also moved to a newer version of their crate, so upgrade that crate along with the rename:

| 2.0 feature | Supported before | Supported in 2.0 |
| --- | --- | --- |
| `bevy_ecs020`, `bevy_input020` | 0.14 (rc.22), 0.18 (rc.25) | 0.20 |
| `bit-vec010` | 0.7 (rc.22), 0.9 (rc.25) | 0.10 |
| `bson3` | 2 (rc.22) | 3 |
| `bytesize2` | 1 (rc.22) | 2 |
| `error-stack08` | 0.7 (rc.25) | 0.8 |
| `glam034` | 0.29 (rc.22), 0.32 (rc.25) | 0.34 |
| `heapless` | 0.7 to 0.9 (rc.25) | 0.9 |
| `ipnetwork021` | 0.20 (rc.22) | 0.21 |
| `jiff1` | 0.1 (rc.22), 0.2 (rc.25) | 1 |
| `toml1` | 0.8 (rc.22) | 1 |
| `uhlc09` | 0.8 (rc.22) | 0.9 |
| `ulid3` | 1 | 3 |

## Core API

| Before | 2.0 | Since |
| --- | --- | --- |
| `TypeMap` (rc.20), `TypeCollection` (rc.22 and rc.23) | `Types`; a deprecated `TypeMap` alias remains for Tauri 2.x | rc.24 |
| `types.register::<T>()` on a `&mut TypeCollection` | `Types::default().register::<T>()`, or `register_mut::<T>()` in place | rc.24 |
| `specta::export()` | `specta::collect()` | rc.23 |
| `#[specta(export = false)]` | `#[specta(collect = false)]` | rc.23 |
| the `Language` trait's `export` | the exporter's own `export` and `export_to` | rc.23 |
| `#[specta::specta]` and `specta::function` | removed; Tauri Specta types commands from Tauri's own metadata | after rc.28 |

## Derive attributes

Serde attributes describe the wire shape now, so the `#[specta(...)]` forms of `rename`, `rename_all`, `tag`, `content`, `untagged`, `flatten`, `skip_serializing`, `skip_deserializing`, and `skip_serializing_if` became errors in rc.23. Each error names its `#[serde(...)]` replacement.

The same release stopped registering `serde` as a helper attribute of `Type`, so a `#[serde(...)]` attribute needs a serde derive on its type. Without one, the compiler reports "cannot find attribute `serde` in this scope".

## TypeScript exporter

| Before | 2.0 | Since |
| --- | --- | --- |
| `export(&types)`, `export_to(path, &types)` | `export(&types, specta_serde::Format)`, `export_to(path, &types, specta_serde::Format)` | rc.25 |
| `.formatter(...)` with `prettier`, `eslint`, or `biome` | run the formatter on the file after `export_to` | rc.23 |
| `.bigint(BigIntExportBehavior::…)` | see [BigInt Forbidden](https://docs.rs/specta-typescript/latest/specta_typescript/struct.Error.html#bigint-forbidden) | rc.24 |
| `.header(...)` | unchanged | |

## Generated output

Regenerate and commit bindings in the same change as the upgrade, because the output changed in several places:

- `serde_json::Value` exports as `Value` instead of `JsonValue`, since rc.23.
- `f32` and `f64` export as `number | null`, since rc.25, because serde_json writes `NaN` and the infinities as `null`. Mark a field you know is finite with `#[specta(type = specta_typescript::Number)]` to keep `number`.
- The file header changed in rc.23, and objects now print one field per line.

## Tauri apps

Tauri Specta changed alongside Specta, so follow its docs for the builder. On Specta's side, commands no longer take `#[specta::specta]`, and Tauri Specta's `Builder::dangerously_cast_bigints_to_number()` replaces `BigIntExportBehavior::Number`.
