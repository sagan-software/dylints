use bevy_ecs::prelude::*;

#[derive(Component)]
struct Enemy(u32);

fn bad(query: Query<&Enemy>) {
    let _count = query.iter().count();
}

fn bad_empty(query: Query<&Enemy>) {
    let _empty = query.is_empty();
}

fn bad_in_closure(query: Query<&Enemy>) {
    let count = || query.iter().count();
    let _count = count();
}

fn bad_get(query: Query<&Enemy>, entity: Entity) {
    let _result = query.get(entity);
}

fn good(query: Query<&Enemy>) {
    let _sum: u32 = query.iter().map(|enemy| enemy.0).sum();
}

fn good_value_in_closure(query: Query<&Enemy>) {
    let _empty = query.is_empty();
    let first = || query.iter().next().map(|enemy| enemy.0);
    let _first = first();
}

fn good_passed(query: Query<&Enemy>) {
    let _empty = query.is_empty();
    take(&query);
}

fn good_mutable(mut query: Query<&mut Enemy>) {
    let _empty = query.is_empty();
    for mut enemy in &mut query {
        enemy.0 += 1;
    }
}

fn good_unnamed(_: Query<&Enemy>) {}

fn take(_: &Query<&Enemy>) {}

trait Census {
    fn count(&self, query: Query<&Enemy>) -> usize;
}

struct EnemyCensus;

impl Census for EnemyCensus {
    fn count(&self, query: Query<&Enemy>) -> usize {
        query.iter().count()
    }
}

fn main() {
    let _bad = bad;
    let _good = good;
}
