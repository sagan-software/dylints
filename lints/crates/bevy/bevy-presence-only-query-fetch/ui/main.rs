use bevy_ecs::prelude::*;

#[derive(Component)]
struct Enemy(u32);

fn bad(query: Query<&Enemy>) {
    let _count = query.iter().count();
}

fn good(query: Query<&Enemy>) {
    let _sum: u32 = query.iter().map(|enemy| enemy.0).sum();
}

fn main() {
    let _bad = bad;
    let _good = good;
}
