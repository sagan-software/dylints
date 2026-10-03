# bevy-conflicting-query-params

## What it does

Warns when a free function registered directly with `App::add_systems` declares overlapping Bevy query access. It checks component access within one query, between query parameters, and between a query and `Res`, `ResMut`, `NonSend`, or `NonSendMut` parameters.

The lint understands borrowed component references, tuples of supported query data, `Entity`, `With`, `Without`, tuple filter conjunctions, `Or`, and `ParamSet` member boundaries. It models Bevy 0.18 and 0.19's built-in `Disabled` default query filter and Bevy 0.19's resource-entity `IsResource` marker.

## Why is this bad?

Bevy may reject overlapping query parameters with B0001 or query-to-resource access with B0002 while it initializes a system. Query data that requests both shared and mutable access to the same component also panics during initialization. These failures can stop an app before the system runs.

## Known problems

This is a static approximation. It skips closures, indirect registrations, custom `SystemParam` implementations, unknown query data, and unknown filters. It expands at most 64 filter alternatives; larger forms are skipped. It recognizes disjointness from `With`, `Without`, tuples, and `Or` only.

Only accesses in different members of the same `ParamSet` are sequential alternatives. Accesses inside one member remain simultaneous, including accesses nested in ordinary tuples. It does not treat contradictory constraints inside one query as proof that the query matches no entities, matching Bevy's filtered-access compatibility behavior.

For Bevy 0.18 and 0.19, the lint assumes the built-in `Disabled` default filter and does not inspect runtime `DefaultQueryFilters` changes. Registering custom disabling components can make a reported query pair disjoint. Replacing the defaults can also make an unreported pair conflict. Query-to-`Res` checks use the resolved ECS crate's `IsResource` marker. A `Without` filter for either `IsResource` or the resource type can also prove disjointness when the same branch does not require that component.

Bevy 0.19 checks `NonSend` access differently. When `NonSend<T>` precedes a query, `Without<T>` can prove disjointness. When the query precedes `NonSend<T>`, Bevy can reject the system despite that filter. `Without<IsResource>` does not disjoin non-send access. The lint tracks supported parameter order and applies this distinction. Bevy 0.18 keeps resource and component access separate, so the lint skips cross-kind checks for that resolved ECS crate.

## Example

```rust,no_run
use bevy_app::{App, Update};
use bevy_ecs::prelude::{Component, Query};

#[derive(Component)]
struct Position;

fn update(write: Query<&mut Position>, read: Query<&Position>) {}

fn main() {
    let mut app = App::new();
    let _configured_app = app.add_systems(Update, update);
}
```

## Use instead

```rust,no_run
use bevy_app::{App, Update};
use bevy_ecs::prelude::{Component, Query, With, Without};

#[derive(Component)]
struct Position;

#[derive(Component)]
struct Selected;

fn update(
    selected: Query<&mut Position, With<Selected>>,
    unselected: Query<&Position, Without<Selected>>,
) {}

fn main() {
    let mut app = App::new();
    let _configured_app = app.add_systems(Update, update);
}
```
