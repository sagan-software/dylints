# bevy-duplicate-plugin-addition

## What it does

Checks for two adjacent `add_plugins` calls in one method chain that add the same local plugin
type, when that plugin does not override `Plugin::is_unique`.

## Why is this bad?

Plugins are unique by default. `App::add_plugins` panics when it adds a unique plugin that the app
already has.

## Known problems

The lint only compares adjacent calls in one chain. It does not report duplicates in separate
statements, in tuples, or in plugin groups. It skips plugin types defined in other crates.

## Example

```rust
fn build(app: &mut App) {
    app.add_plugins(GamePlugin).add_plugins(GamePlugin);
}
```

## Use instead

```rust
fn build(app: &mut App) {
    app.add_plugins(GamePlugin);
}
```
