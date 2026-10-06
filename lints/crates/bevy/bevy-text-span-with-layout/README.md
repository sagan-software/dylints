# bevy-text-span-with-layout

## What it does

Warns when a direct Bevy ECS tuple bundle passed to a recognized `spawn`, `insert`, `insert_if_new`, or related-spawner `spawn` method contains both Bevy `TextSpan` and `TextLayout` components.

## Why is this bad?

Bevy's text processing emits a once-only warning when it processes a changed `TextSpan` entity that also has `TextLayout`. `TextLayout` belongs on a root text entity with `Text` or `Text2d`.

## Known problems

The lint recognizes direct tuple bundles, including nested tuples. It does not inspect custom bundles, local variables, or combinations assembled across separate calls. It reports the component combination in one bundle expression and does not prove the entity's final hierarchy or configuration. Bevy's text-processing system must run on the span for its runtime warning to occur.

The lint does not evaluate whether `insert_if_new` applies a bundle to the current entity.

The runtime behavior here is checked against [Bevy 0.19.0's text processing source](https://github.com/bevyengine/bevy/blob/v0.19.0/crates/bevy_text/src/text.rs). Later Bevy versions may change the warning or its conditions.

## Example

```rust
# use bevy::prelude::*;
use bevy::ecs::world::World;
use bevy::text::{TextLayout, TextSpan};

fn main() {
    let mut world = World::new();
    world.spawn((TextSpan::new("name"), TextLayout::default()));
}
```

## Use instead

Put `TextLayout` on the root entity with `Text` or `Text2d`, then parent each span under that root.

```rust
# use bevy::prelude::*;
use bevy::ecs::{hierarchy::ChildOf, world::World};
use bevy::text::{TextLayout, TextSpan};
use bevy::ui::widget::Text;

fn main() {
    let mut world = World::new();
    let root = world
        .spawn((Text::new("Name plate"), TextLayout::default()))
        .id();
    let child = world.spawn(TextSpan::new("Player")).id();
    world.entity_mut(child).insert(ChildOf(root));
}
```
