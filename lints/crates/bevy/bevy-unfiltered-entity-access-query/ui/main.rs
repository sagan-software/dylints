use bevy_ecs::prelude::*;

#[derive(Component)]
struct Position(f32);

fn bad(query: Query<EntityRef>) {
    for entity in &query {
        let _position = entity.get::<Position>();
    }
}

fn good(query: Query<&Position>) {
    for position in &query {
        let _x = position.0;
    }
}

fn main() {
    let _bad = bad;
    let _good = good;
}
