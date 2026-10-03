#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unused_assignments,
    unused_crate_dependencies,
    unknown_lints,
    unused_variables,
    clippy::missing_docs_in_private_items,
    clippy::useless_let_if_seq,
    reason = "This fixture keeps each lint case small and independent."
)]

use bevy_asset::{AssetServer, DirectAssetAccessExt, LoadBuilder};
use bevy_ecs::world::World;

struct Guard;

struct BuilderSlot<'a> {
    builder: LoadBuilder<'a>,
}

fn direct_chain(asset_server: &AssetServer) {
    let _builder = asset_server
        .load_builder()
        .with_guard(Guard)
        .with_guard(Guard);
}

fn let_else_chain_is_checked(asset_server: &AssetServer, load: bool) {
    let Some(()) = load.then_some(()) else {
        let _builder = asset_server
            .load_builder()
            .with_guard(Guard)
            .with_guard(Guard);
        return;
    };
}

fn local_move(asset_server: &AssetServer) {
    let builder = asset_server.load_builder().with_guard(Guard);
    let builder = builder.with_guard(Guard);
    let _builder = builder;
}

fn call_between_guards(asset_server: &AssetServer) {
    let _builder = asset_server
        .load_builder()
        .with_guard(Guard)
        .with_settings(|_settings: &mut ()| {})
        .with_guard(Guard);
}

fn replace_after_option(asset_server: &AssetServer) {
    let _builder = asset_server
        .load_builder()
        .with_guard(Guard)
        .override_unapproved()
        .with_guard(Guard);
}

fn one_guard(asset_server: &AssetServer) {
    let _builder = asset_server.load_builder().with_guard((Guard, Guard));
}

fn separate_builders(asset_server: &AssetServer) {
    let _first = asset_server.load_builder().with_guard(Guard);
    let _second = asset_server.load_builder().with_guard(Guard);
}

fn unrelated_assignment_preserves_builder_identity(asset_server: &AssetServer) {
    let builder = asset_server.load_builder().with_guard(Guard);
    let mut revision = 0;
    assert_eq!(revision, 0);
    revision = 1;
    assert_eq!(revision, 1);
    let _guarded_builder = builder.with_guard(Guard);
}

fn reassigned_builder_is_not_tracked(asset_server: &AssetServer) {
    let mut builder = asset_server.load_builder().with_guard(Guard);
    builder = asset_server.load_builder();
    let _builder = builder.with_guard(Guard);
}

fn conditionally_reassigned_builder_is_not_tracked(asset_server: &AssetServer, replace: bool) {
    let mut builder = asset_server.load_builder().with_guard(Guard);
    if replace {
        builder = asset_server.load_builder();
    }
    let _builder = builder.with_guard(Guard);
}

fn field_builder_assignment_is_not_followed(asset_server: &AssetServer) {
    let mut slot = BuilderSlot {
        builder: asset_server.load_builder().with_guard(Guard),
    };
    slot.builder = asset_server.load_builder().with_guard(Guard);
    let _builder = slot.builder.with_guard(Guard);
}

fn builder_from_helper(asset_server: &AssetServer) -> LoadBuilder<'_> {
    asset_server.load_builder()
}

fn helper_builder_origin_is_not_assumed(asset_server: &AssetServer) {
    let builder = builder_from_helper(asset_server).with_guard(Guard);
    let _builder = builder.with_guard(Guard);
}

fn world_builder_is_not_tracked(world: &World) {
    let _builder = world.load_builder().with_guard(Guard).with_guard(Guard);
}

fn replace_builder_through_helper<'a>(
    builder: &mut LoadBuilder<'a>,
    asset_server: &'a AssetServer,
) {
    *builder = asset_server.load_builder();
}

fn mutable_builder_escape_invalidates_guard_facts(asset_server: &AssetServer) {
    let mut builder = asset_server.load_builder().with_guard(Guard);
    replace_builder_through_helper(&mut builder, asset_server);
    let _builder = builder.with_guard(Guard);
}

struct BuilderMutator {
    should_replace: bool,
}

impl BuilderMutator {
    fn replace_builder<'a>(&self, builder: &mut LoadBuilder<'a>, asset_server: &'a AssetServer) {
        if self.should_replace {
            *builder = asset_server.load_builder();
        }
    }
}

fn mutable_builder_method_argument_invalidates_guard_facts(asset_server: &AssetServer) {
    let mut builder = asset_server.load_builder().with_guard(Guard);
    BuilderMutator {
        should_replace: true,
    }
    .replace_builder(&mut builder, asset_server);
    let _builder = builder.with_guard(Guard);
}

fn invoke_opaque_action<F: FnMut()>(mut action: F) {
    action();
}

struct ActionRunner {
    should_run: bool,
}

impl ActionRunner {
    fn run<F: FnMut()>(&self, mut action: F) {
        if self.should_run {
            action();
        }
    }
}

fn called_capturing_closure_invalidates_guard_facts(asset_server: &AssetServer) {
    let mut builder = asset_server.load_builder().with_guard(Guard);
    let mut replace = || builder = asset_server.load_builder();
    replace();
    let _builder = builder.with_guard(Guard);
}

fn passed_capturing_closure_invalidates_guard_facts(asset_server: &AssetServer) {
    let mut builder = asset_server.load_builder().with_guard(Guard);
    invoke_opaque_action(|| builder = asset_server.load_builder());
    let _builder = builder.with_guard(Guard);
}

fn method_closure_argument_invalidates_guard_facts(asset_server: &AssetServer) {
    let mut builder = asset_server.load_builder().with_guard(Guard);
    ActionRunner { should_run: true }.run(|| builder = asset_server.load_builder());
    let _builder = builder.with_guard(Guard);
}

fn uninvoked_closure_preserves_guard_facts(asset_server: &AssetServer) {
    let mut builder = asset_server.load_builder().with_guard(Guard);
    {
        let replace = || builder = asset_server.load_builder();
    }
    let _builder = builder.with_guard(Guard);
}

fn nonlocal_builder_reborrow_escape_clears_guard_facts(asset_server: &AssetServer) {
    let mut builder = asset_server.load_builder().with_guard(Guard);
    let builder_reference = &mut builder;
    replace_builder_through_helper(&mut *builder_reference, asset_server);
    let _builder = builder.with_guard(Guard);
}

fn main() {}
