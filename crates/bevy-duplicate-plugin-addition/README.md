# bevy-duplicate-plugin-addition

## What it does

Checks for repeated additions of the same local plugin type when its `Plugin` implementation uses
the default `is_unique` behavior. It checks adjacent calls in one chain, repeated types inside a
tuple, and separate direct calls on one local `App` or `SubApp` within a block.

## Why is this bad?

Plugins are unique by default. `App::add_plugins` panics when it adds a unique plugin that the app
already has.

## Known problems

The lint resolves local plugin implementations and direct calls on local `App` or `SubApp` values.
It does not infer uniqueness for external plugins, plugin groups, or local plugins that override
`is_unique`. Separate-call analysis forgets its history when an intervening statement uses the app,
so mutations, aliases, and reassignment stop the analysis. It does not trace app values across
blocks or control-flow paths.

When local implementations share a generic plugin ADT, the lint resolves each concrete
instantiation separately, such as `GenericPlugin<u8>` and `GenericPlugin<u16>`. If it cannot select
one implementation for an instantiation, it leaves that type unclassified. Tuple and separate-call
diagnostics provide help without a machine fix because removing an expression could change its side
effects. A chained-call machine fix is limited to a repeated path argument with source spans
outside macro expansions.

## Example

```rust
use bevy::app::{App, Plugin};

struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, _: &mut App) {}
}

struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, _: &mut App) {}
}

fn build(app: &mut App) {
    app.add_plugins(GamePlugin).add_plugins(GamePlugin);
    app.add_plugins((AudioPlugin, AudioPlugin));
}
```

## Use instead

```rust
use bevy::app::{App, Plugin};

struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, _: &mut App) {}
}

struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, _: &mut App) {}
}

fn build(app: &mut App) {
    app.add_plugins(GamePlugin);
    app.add_plugins(AudioPlugin);
}
```
