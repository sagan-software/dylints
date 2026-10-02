use bevy_ecs::prelude::*;

#[derive(Component)]
struct Position(f32);

fn bad(query: Query<EntityRef>) {
    for entity in &query {
        let _position = entity.get::<Position>();
    }
}

fn bad_mut(mut query: Query<EntityMut>) {
    query.iter_mut().for_each(|mut entity| {
        if let Some(mut position) = entity.get_mut::<Position>() {
            position.0 += 1.0;
        }
    });
}

fn good(query: Query<&Position>) {
    for position in &query {
        let _x = position.0;
    }
}

fn good_dynamic(query: Query<EntityRef>) {
    for entity in &query {
        let _id = entity.id();
    }
}

fn main() {
    let _bad = bad;
    let _good = good;
}
