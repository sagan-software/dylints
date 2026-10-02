#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::{
    change_detection::{Mut, MutUntyped, NonSendMut, ResMut},
    component::Component,
    ptr::PtrMut,
    resource::Resource,
    system::{Commands, Deferred, EntityCommands, Query},
    world::{CommandQueue, DeferredWorld, EntityMut, FilteredEntityMut},
};

#[derive(Component)]
struct Marker;

#[derive(Resource)]
struct Score(u32);

fn bad(_: &mut Query<&Marker>) {}
fn bad_commands(_: &mut Commands) {}
fn bad_deferred(_: &mut Deferred<CommandQueue>) {}
fn bad_deferred_world(_: &mut DeferredWorld) {}
fn bad_entity_commands(_: &mut EntityCommands) {}
fn bad_entity_mut(_: &mut EntityMut) {}
fn bad_filtered_entity_mut(_: &mut FilteredEntityMut) {}
fn bad_mut(_: &mut Mut<Marker>) {}
fn bad_mut_untyped(_: &mut MutUntyped) {}
fn bad_non_send_mut(_: &mut NonSendMut<Score>) {}
fn bad_ptr_mut(_: &mut PtrMut) {}
fn bad_res_mut(_: &mut ResMut<Score>) {}

fn bad_closure() {
    let _closure = |_: &mut Commands| {};
}

fn good(_: Query<&Marker>) {}
fn good_shared(_: &Commands) {}
fn good_other(_: &mut u32) {}

fn good_returned<'a, 'w, 's>(commands: &'a mut Commands<'w, 's>) -> &'a mut Commands<'w, 's> {
    commands
}

trait Spawner {
    fn spawn_with(&self, commands: &mut Commands);
}

struct MarkerSpawner;

impl Spawner for MarkerSpawner {
    fn spawn_with(&self, _: &mut Commands) {}
}

impl MarkerSpawner {
    fn bad_method(&mut self, _: &mut Commands) {}
}

fn main() {}
