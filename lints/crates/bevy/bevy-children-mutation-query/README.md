# bevy-children-mutation-query

## What it does

Checks for function parameters of type `Query<..>` whose query data contains `&mut Children`.

## Why is this bad?

`Children` is the relationship target of `ChildOf`. Bevy updates it when `ChildOf` changes on the
child entities. Editing `Children` directly can leave it out of sync with the `ChildOf` components.

## Known problems

Resolved inherent `Children` reordering methods such as `swap` and `sort_by` are exempt, including
calls inside closures, when no other mutable `Children` use appears. Direct collection mutation, a
mutable handle passed elsewhere, and a local `&mut Children` reborrow or alias still trigger a
warning, even when that local value only calls `sort_by`.

The query analysis does not look inside `Option<&mut Children>` or custom `QueryData` types.

## Example

```rust
# use bevy_ecs::{hierarchy::Children, prelude::Query, relationship::RelationshipTarget};

fn detach_all(mut parents: Query<&mut Children>) {
    for mut children in &mut parents {
        children.collection_mut_risky().clear();
    }
}
```

## Use instead

```rust
# use bevy_ecs::{hierarchy::Children, prelude::{Commands, Entity, Query, With}};

fn detach_all(mut commands: Commands, parents: Query<Entity, With<Children>>) {
    for parent in &parents {
        commands.entity(parent).detach_all_children();
    }
}
```
