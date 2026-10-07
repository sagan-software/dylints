# bevy-asset-source-after-asset-plugin

## What it does

Checks direct `AssetApp::register_asset_source` calls after an explicit `AssetPlugin` addition on one tracked `App` value. It also checks explicit `WebAssetPlugin` order in direct plugin tuples and chained `App::add_plugins` calls.

## Why is this bad?

Bevy builds asset sources when `AssetPlugin` starts. A later source registration logs an error and cannot update the active `AssetServer`. `WebAssetPlugin` must be added first so it can register its HTTP sources.

## Known problems

The lint follows resolved direct plugin values, tuple elements in source order, and direct local `App` aliases within one function body. It preserves app identity through direct `add_plugins` and `register_asset_source` chains. It skips `DefaultPlugins` and other plugin groups because the compiled group's member list is not available through the resolved call, and group operations can disable or reorder members. It also skips custom plugin bodies, helper calls, unrelated app methods in a chain, and aliases it cannot prove are the same value.

At branch joins, it keeps only facts shared by every path. A branch-only plugin addition does not affect later checks. If an `App` binding may refer to different values across paths or receives an unproven reassignment, the lint stops tracking that binding. A later assignment from a proven local app can restore tracking.

The lint invalidates plugin facts when a tracked app escapes through a mutable-reference argument to an opaque call. Implicit mutable method receivers are not analyzed. It cannot inspect the helper's behavior, so later ordering mistakes can remain unreported.

The lint clears tracked facts when it invokes a closure value or passes one to an opaque function or method. It also clears them for an opaque method invoked on a closure receiver. This conservative handling does not inspect captures, so an unrelated closure can suppress later warnings. Creating and dropping a closure without invoking or passing it preserves tracking.

## Example

```no_run
# use bevy::app::App;
# use bevy::asset::{
#     AssetApp, AssetPlugin,
#     io::{AssetSourceBuilder, memory::MemoryAssetReader},
# };
# fn source_builder() -> AssetSourceBuilder {
#     AssetSourceBuilder::new(|| Box::new(MemoryAssetReader::default()))
# }
# fn example(app: &mut App) {
app.add_plugins(AssetPlugin::default());
app.register_asset_source("remote", source_builder());
# }
```

## Use instead

```no_run
# use bevy::app::App;
# use bevy::asset::{
#     AssetApp, AssetPlugin,
#     io::{AssetSourceBuilder, memory::MemoryAssetReader},
# };
# fn source_builder() -> AssetSourceBuilder {
#     AssetSourceBuilder::new(|| Box::new(MemoryAssetReader::default()))
# }
# fn example(app: &mut App) {
app.register_asset_source("remote", source_builder());
app.add_plugins(AssetPlugin::default());
# }
```
