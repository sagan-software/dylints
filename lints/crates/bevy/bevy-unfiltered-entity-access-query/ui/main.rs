use bevy_ecs::prelude::*;

#[derive(Component)]
struct Position(f32);

fn inspect_entity(entity: EntityRef<'_>) {
    let _position = entity.get::<Position>();
}

macro_rules! access_position {
    ($entity:expr) => {
        $entity.get::<Position>()
    };
}

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

fn good_dynamic(query: Query<EntityRef>, entity_id: Entity) {
    for entity in &query {
        let _id = entity.id();
        if entity.id() != entity_id {
            let _id = entity.id();
        }
    }
}

fn bad_query_get(query: Query<EntityRef>, entity_id: Entity) {
    let entity = query.get(entity_id).unwrap();
    let _position = entity.get::<Position>();
}

fn bad_query_get_mut(mut query: Query<EntityMut>, entity_id: Entity) {
    let _position = query.get_mut(entity_id).unwrap().get_mut::<Position>();
}

fn bad_query_single(query: Query<EntityRef>) {
    let _position = query.single().unwrap().get::<Position>();
}

fn bad_query_single_mut(mut query: Query<EntityMut>) {
    let _position = query.single_mut().unwrap().get_mut::<Position>();
}

fn bad_query_iter_next(query: Query<EntityRef>) {
    let _position = query.iter().next().unwrap().get::<Position>();
}

fn bad_query_get_if_let(query: Query<EntityRef>, entity_id: Entity) {
    if let Ok(entity) = query.get(entity_id) {
        let _position = entity.get::<Position>();
    }
}

fn bad_query_get_match(query: Query<EntityRef>, entity_id: Entity) {
    match query.get(entity_id) {
        Ok(entity) => {
            let _position = entity.get::<Position>();
        }
        Err(_) => {}
    }
}

fn bad_query_single_expect(query: Query<EntityRef>) {
    let _position = query.single().expect("one entity").get::<Position>();
}

fn false_positive(world: &World, entity_id: Entity, query: Query<EntityRef>) {
    let _id = query.iter().next().map(|entity| entity.id());
    let _position = world.get_entity(entity_id).unwrap().get::<Position>();
}

fn shadowed_entity(world: &World, entity_id: Entity, query: Query<EntityRef>) {
    for entity in &query {
        let _id = entity.id();
        let entity = world.get_entity(entity_id).unwrap();
        let _position = entity.get::<Position>();
    }
}

fn overwritten_entity<'w>(
    world: &'w World,
    entity_id: Entity,
    query: Query<'w, '_, EntityRef<'w>>,
) {
    for mut entity in &query {
        let _id = entity.id();
        entity = world.get_entity(entity_id).unwrap();
        let _position = entity.get::<Position>();
    }
}

fn bad_query_borrow(query: Query<EntityRef>) {
    for entity in &query {
        let _position = (&entity).get::<Position>();
    }
}

fn bad_query_macro(query: Query<EntityRef>) {
    for entity in &query {
        let _position = access_position!(entity);
    }
}

fn unknown_query_adapter(query: Query<EntityRef>) {
    let _position = query
        .iter()
        .map(|entity| entity)
        .next()
        .unwrap()
        .get::<Position>();
}

fn unknown_named_for_each(query: Query<EntityRef>) {
    query.iter().for_each(inspect_entity);
}

fn two_queries(dynamic_query: Query<EntityRef>, typed_query: Query<EntityRef>) {
    for entity in &dynamic_query {
        let _id = entity.id();
    }
    typed_query.iter().for_each(|entity| {
        let _position = entity.get::<Position>();
    });
}

fn tuple_query_data(query: Query<(Entity, EntityRef)>) {
    for (_entity_id, entity) in &query {
        let _position = entity.get::<Position>();
    }
}

fn rhs_access_before_overwrite<'w>(w: &'w World, id: Entity, q: Query<'w, '_, EntityRef<'w>>) {
    for mut entity in &q {
        entity = {
            let _position = entity.get::<Position>();
            w.get_entity(id).unwrap()
        };
        let _id = entity.id();
    }
}

fn return_query<'w, 's>(query: Query<'w, 's, EntityRef<'w>>) -> Query<'w, 's, EntityRef<'w>> {
    query
}

fn unknown_query_iterator_receiver<'w>(query: Query<'w, '_, EntityRef<'w>>) {
    for entity in return_query(query).iter() {
        let _position = entity.get::<Position>();
    }
}

fn bad_query_iter_mut_next(mut query: Query<EntityMut>) {
    let _position = query.iter_mut().next().unwrap().get_mut::<Position>();
}

fn bad_query_get_let_else(query: Query<EntityRef>, entity_id: Entity) {
    let Ok(entity) = query.get(entity_id) else {
        return;
    };
    let _position = entity.get::<Position>();
}

fn unknown_query_get_receiver<'w>(query: Query<'w, '_, EntityRef<'w>>, entity_id: Entity) {
    let _position = return_query(query)
        .get(entity_id)
        .unwrap()
        .get::<Position>();
}

fn main() {
    let _bad = bad;
    let _good = good;
}
