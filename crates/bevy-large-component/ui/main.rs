use bevy::ecs::prelude::*;

#[derive(Component)]
struct LargeAgent {
    a: u64,
    b: u64,
    c: u64,
    d: u64,
    e: u64,
    f: u64,
    g: u64,
    h: u64,
    i: u64,
}

#[derive(Component)]
struct CohesiveTransform {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Resource)]
struct LargeResource {
    a: u64,
    b: u64,
    c: u64,
    d: u64,
    e: u64,
    f: u64,
    g: u64,
    h: u64,
    i: u64,
}

fn main() {
    let _ = core::mem::size_of::<LargeAgent>();
    let _ = core::mem::size_of::<CohesiveTransform>();
    let _ = core::mem::size_of::<LargeResource>();
}
