# bevy-children-mutation-query

## What it does

Checks for function parameters of type `Query<..>` whose query data contains `&mut Children`.

## Why is this bad?

`Children` is the relationship target of `ChildOf`. Bevy updates it when `ChildOf` changes on the
child entities. Editing `Children` directly can leave it out of sync with the `ChildOf` components.

## Known problems

Bevy provides `Children::swap` and the `Children::sort_by` methods to reorder children through
`&mut Children`. These keep the hierarchy in sync, but the lint also reports queries that only use
them.

The lint does not look inside `Option<&mut Children>` or custom `QueryData` types.

## Example

```rust
use bevy::ecs::relationship::RelationshipTarget;

fn detach_all(mut parents: Query<&mut Children>) {
    for mut children in &mut parents {
        children.collection_mut_risky().clear();
    }
}
```

## Use instead

```rust
fn detach_all(mut commands: Commands, parents: Query<Entity, With<Children>>) {
    for parent in &parents {
        commands.entity(parent).detach_all_children();
    }
}
```
