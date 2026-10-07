# bevy-duplicate-dependencies

## What it does

Checks for crates that load two or more crates named `bevy`, for example two versions of the Bevy
facade crate.

## Why is this bad?

Each Bevy version defines its own `Component`, `Plugin`, and other traits and types. Types from one
version do not work with the other, which causes confusing type errors. The extra copy also adds
compile time and binary size.

## Known problems

The lint only counts crates named `bevy`. It does not report duplicate versions of subcrates such
as `bevy_ecs` when code loads only one `bevy` facade. It reports the whole crate, not the dependency
entry in `Cargo.toml`.

## Example

```toml
[dependencies]
bevy = "0.19.0"
bevy_018 = { package = "bevy", version = "0.18.0" }
```

## Use instead

```toml
[dependencies]
bevy = "0.19.0"
```
