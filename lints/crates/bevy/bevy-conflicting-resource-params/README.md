# bevy-conflicting-resource-params

## What it does

Warns when a free function registered directly with `App::add_systems` declares conflicting `Res`, `ResMut`, `NonSend`, or `NonSendMut` access to one resolved type. It follows direct parameters, ordinary nested tuples, and `ParamSet` members.

## Why is this bad?

Bevy rejects conflicting resource access with B0002 while it initializes a system. A conflict can therefore stop an app before the system runs.

## Known problems

The lint skips closures, indirect registrations, custom `SystemParam` implementations, and wrappers around supported parameters. It compares resolved resource types and reports conflicts outside a `ParamSet` or within one member. Only different members of the same `ParamSet` are sequential alternatives; accesses inside one member remain simultaneous, including accesses nested in ordinary tuples. This lint checks resource-to-resource access. Query-to-resource conflicts are checked by `bevy-conflicting-query-params`.

## Example

```rust,no_run
use bevy_app::{App, Update};
use bevy_ecs::prelude::{Res, ResMut, Resource};

#[derive(Resource)]
struct State;

fn update(mut write: ResMut<State>, read: Res<State>) {}

fn main() {
    let mut app = App::new();
    let _configured_app = app.add_systems(Update, update);
}
```

## Use instead

```rust,no_run
use bevy_app::{App, Update};
use bevy_ecs::prelude::{ResMut, Resource};

#[derive(Resource)]
struct State;

fn update(mut state: ResMut<State>) {}

fn main() {
    let mut app = App::new();
    let _configured_app = app.add_systems(Update, update);
}
```
