# bevy-load-builder-replaced-guard

## What it does

Checks, within one function body, for repeated `LoadBuilder::with_guard` calls on one builder returned directly by `AssetServer::load_builder()`.

## Why is this bad?

Bevy keeps only the last guard. Replacing an earlier guard drops it before loading starts, which can signal completion too early.

## Known problems

The lint follows direct builder chains, `with_settings` and `override_unapproved` calls, and local moves whose value identity is clear. It does not follow other builder origins, including `World::load_builder()` or `LoadContext::load_builder()`, opaque helpers, assignments to mutable builders, or aliases it cannot prove. Replacing a guard can be intentional.

The lint invalidates guard facts when a tracked builder escapes through a mutable-reference argument to an opaque call. Implicit mutable method receivers are not analyzed. It cannot inspect the helper's behavior, so later guard replacements can remain unreported.

The lint clears tracked facts when it invokes a closure value or passes one to an opaque function or method. It also clears them for an opaque method invoked on a closure receiver. This conservative handling does not inspect captures, so an unrelated closure can suppress later warnings. Creating and dropping a closure without invoking or passing it preserves tracking.

## Example

```no_run
# use bevy_asset::AssetServer;
# fn example(asset_server: &AssetServer) {
let _handle = asset_server
    .load_builder()
    .with_guard(())
    .with_guard(())
    .load_untyped("assets/example.png");
# }
```

## Use instead

```no_run
# use bevy_asset::AssetServer;
# fn example(asset_server: &AssetServer) {
let _handle = asset_server
    .load_builder()
    .with_guard(((), ()))
    .load_untyped("assets/example.png");
# }
```
