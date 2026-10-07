//! UI fixture for supported conflicting and non-conflicting Bevy system parameters.

#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    unused_crate_dependencies,
    unused_variables
)]

use bevy::app::{App, Update};
use bevy::ecs::entity_disabling::Disabled;
use bevy::ecs::prelude::*;
use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::ecs::resource::IsResource;
use bevy_018 as old_bevy;
use bevy_018_system_fixture::OldPosition;

mod lookalike_query {
    use bevy::ecs::component::Mutable;
    use bevy::ecs::{
        prelude::{ResMut, Resource},
        system::SystemParam,
    };
    use std::marker::PhantomData;

    #[derive(SystemParam)]
    pub(crate) struct Query<'w, 's, T: Resource<Mutability = Mutable>> {
        resource: ResMut<'w, T>,
        state: PhantomData<&'s ()>,
    }
}

#[derive(Component)]
struct Position;

#[derive(Component)]
struct OtherPosition;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

#[derive(Resource)]
struct SharedResource;

#[derive(Component)]
struct LocalValue;

#[derive(QueryData)]
#[query_data(mutable)]
struct CustomPosition {
    position: &'static mut Position,
}

#[derive(QueryFilter)]
struct CustomPlayerFilter {
    player: With<Player>,
}

type TwoWayFilter = Or<(With<Player>, With<Enemy>)>;
type FourWayFilter = Or<(TwoWayFilter, TwoWayFilter)>;
type EightWayFilter = Or<(FourWayFilter, FourWayFilter)>;
type SixteenWayFilter = Or<(EightWayFilter, EightWayFilter)>;
type ThirtyTwoWayFilter = Or<(SixteenWayFilter, SixteenWayFilter)>;
type SixtyFourWayFilter = Or<(ThirtyTwoWayFilter, ThirtyTwoWayFilter)>;
type OneHundredTwentyEightWayFilter = Or<(SixtyFourWayFilter, SixtyFourWayFilter)>;
type ProductAboveLimitFilter = (SixtyFourWayFilter, TwoWayFilter);

fn conflicting(write: Query<&mut Position>, read: Query<&Position>) {}

fn conflicting_inside_one_param_set_member(
    params: ParamSet<
        '_,
        '_,
        ((
            Query<'_, '_, &'static mut Position>,
            Query<'_, '_, &'static Position>,
        ),),
    >,
) {
}

fn conflicting_with_overlapping_filters(
    player: Query<&mut Position, With<Player>>,
    enemy: Query<&Position, With<Enemy>>,
) {
}

fn conflicting_or_branch(
    marked: Query<&mut Position, Or<(With<Player>, With<Enemy>)>>,
    players: Query<&Position, With<Player>>,
) {
}

fn conflicting_duplicate_filter_constraint(
    write: Query<&mut Position, (With<Player>, With<Player>)>,
    read: Query<&Position, With<Player>>,
) {
}

fn conflicting_query_data(query: Query<(&mut Position, &Position), Or<()>>) {}

fn conflicting_external_to_param_set(
    params: ParamSet<(Query<&mut Position>, Query<&Position>)>,
    external: Query<&Position>,
) {
}

fn conflicting_resource_pair(query: Query<&mut SharedResource>, resource: ResMut<SharedResource>) {}

fn conflicting_non_send_pair(query: Query<&mut LocalValue>, resource: NonSendMut<LocalValue>) {}

fn conflicting_non_send_type_filter_query_first(
    query: Query<&mut LocalValue, Without<LocalValue>>,
    resource: NonSendMut<LocalValue>,
) {
}

fn conflicting_non_send_resource_marker_filter(
    resource: NonSendMut<LocalValue>,
    query: Query<&mut LocalValue, Without<IsResource>>,
) {
}

fn conflicting_ordinary_tuple(parameters: ((Query<&mut Position>, Query<&Position>),)) {}

fn valid_filter_complements(
    players: Query<&mut Position, (With<Player>, Without<Enemy>)>,
    enemies: Query<&Position, With<Enemy>>,
) {
}

fn valid_or_complement(
    marked: Query<&mut Position, Or<(With<Player>, With<Enemy>)>>,
    unmarked: Query<&Position, (Without<Player>, Without<Enemy>)>,
) {
}

fn valid_empty_or_filter(first: Query<&mut Position, Or<()>>, second: Query<&Position>) {}

fn valid_param_set_alternatives(params: ParamSet<(Query<&mut Position>, Query<&Position>)>) {}

fn valid_shared_queries(first: Query<&Position>, second: Query<&Position>) {}

fn valid_distinct_components(first: Query<&mut Position>, second: Query<&mut OtherPosition>) {}

fn valid_query_and_resource(
    query: Query<&mut SharedResource, Without<IsResource>>,
    resource: ResMut<SharedResource>,
) {
}

fn valid_query_and_resource_type_filter(
    query: Query<&mut SharedResource, Without<SharedResource>>,
    resource: ResMut<SharedResource>,
) {
}

fn valid_preceding_non_send_type_filter(
    resource: NonSendMut<LocalValue>,
    query: Query<&mut LocalValue, Without<LocalValue>>,
) {
}

fn unsupported_query_lookalike(
    fake: lookalike_query::Query<'_, '_, SharedResource>,
    real: Query<&mut SharedResource>,
) {
}

fn valid_default_disabled_filter(
    disabled: Query<&mut Position, With<Disabled>>,
    enabled: Query<&Position>,
) {
}

fn valid_entity_queries(entities: Query<Entity>, positions: Query<&Position>) {}

fn valid_explicit_disabled_data(disabled: Query<&mut Disabled>, positions: Query<&Position>) {}

fn unsupported_custom_query_data(custom: Query<CustomPosition>, ordinary: Query<&Position>) {}

fn unsupported_custom_filter(
    custom: Query<&mut Position, CustomPlayerFilter>,
    ordinary: Query<&Position, With<Player>>,
) {
}

fn unsupported_large_filter(
    broad: Query<&mut Position, OneHundredTwentyEightWayFilter>,
    ordinary: Query<&Position>,
) {
}

fn unsupported_filter_product_above_limit(query: Query<&Position, ProductAboveLimitFilter>) {}

fn unsupported_changed_filter(query: Query<&mut Position, Changed<Position>>) {}

fn valid_query_and_distinct_resource(
    query: Query<&mut Position>,
    resource: ResMut<SharedResource>,
) {
}

fn valid_query_resource_param_set(
    params: ParamSet<(Query<&mut SharedResource>, ResMut<SharedResource>)>,
) {
}

fn valid_empty_param_set(params: ParamSet<Vec<Query<&mut Position>>>) {}

fn unregistered_helper(write: Query<&mut Position>, read: Query<&Position>) {}

fn old_resource_component_pair(
    _: old_bevy::ecs::system::Res<'_, OldPosition>,
    _: old_bevy::ecs::system::Query<'_, '_, &'static mut OldPosition>,
) {
}

fn old_default_disabled_filters_disjoint_queries(
    _: old_bevy::ecs::system::Query<
        '_,
        '_,
        &'static mut OldPosition,
        old_bevy::ecs::query::With<old_bevy::ecs::entity_disabling::Disabled>,
    >,
    _: old_bevy::ecs::system::Query<'_, '_, &'static OldPosition>,
) {
}

fn main() {
    let mut app = App::new();
    let _conflict_cases = app.add_systems(
        Update,
        (
            conflicting,
            conflicting_inside_one_param_set_member,
            conflicting_with_overlapping_filters,
            conflicting_or_branch,
            conflicting_duplicate_filter_constraint,
            conflicting_query_data,
            conflicting_external_to_param_set,
            conflicting_resource_pair,
            conflicting_non_send_pair,
            conflicting_non_send_type_filter_query_first,
            conflicting_non_send_resource_marker_filter,
            conflicting_ordinary_tuple,
        ),
    );
    let _valid_cases = app.add_systems(
        Update,
        (
            valid_filter_complements,
            valid_or_complement,
            valid_empty_or_filter,
            valid_param_set_alternatives,
            valid_shared_queries,
            valid_distinct_components,
            valid_query_and_resource,
            valid_query_and_resource_type_filter,
            valid_preceding_non_send_type_filter,
            unsupported_query_lookalike,
            valid_default_disabled_filter,
            valid_entity_queries,
            valid_explicit_disabled_data,
            unsupported_custom_query_data,
            unsupported_custom_filter,
            unsupported_large_filter,
        ),
    );
    let _additional_valid_cases = app.add_systems(
        Update,
        (
            unsupported_filter_product_above_limit,
            unsupported_changed_filter,
            valid_query_and_distinct_resource,
            valid_query_resource_param_set,
            valid_empty_param_set,
        ),
    );

    let _closure = app.add_systems(Update, |_: Query<&mut Position>, _: Query<&Position>| {});

    let mut old_app = old_bevy::app::App::new();
    let _old_app = old_app.add_systems(
        old_bevy::app::Update,
        (
            old_resource_component_pair,
            old_default_disabled_filters_disjoint_queries,
        ),
    );
}
