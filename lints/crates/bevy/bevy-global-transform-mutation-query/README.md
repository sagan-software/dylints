# bevy-global-transform-mutation-query

## What it does

Checks for function parameters of type `Query<..>` whose query data contains
`&mut GlobalTransform`.

## Why is this bad?

Bevy computes `GlobalTransform` from `Transform` and the hierarchy during transform propagation.
A direct write makes `GlobalTransform` disagree with `Transform`, and the next propagation for that
entity overwrites it.

## Known problems

The lint does not look inside `Option<&mut GlobalTransform>` or custom `QueryData` types.

## Example

```rust
fn raise(mut query: Query<&mut GlobalTransform, With<Player>>) {
    for mut transform in &mut query {
        *transform = transform.mul_transform(Transform::from_xyz(0.0, 1.0, 0.0));
    }
}
```

## Use instead

```rust
fn raise(mut query: Query<&mut Transform, With<Player>>) {
    for mut transform in &mut query {
        transform.translation.y += 1.0;
    }
}
```
