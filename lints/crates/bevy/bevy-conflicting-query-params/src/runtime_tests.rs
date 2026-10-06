//! Runtime checks for query-access conflicts reported by Bevy 0.18 and 0.19.

use bevy::ecs::{
    component::Component,
    entity_disabling::{DefaultQueryFilters, Disabled},
    prelude::{NonSendMut, Or, ParamSet, Query, ResMut, Resource, With, Without},
    resource::IsResource,
    system::SystemState,
    world::World,
};
use bevy::reflect::Reflect;
use bevy_018 as old_bevy;
use bevy_018_system_fixture::OldPosition;
use std::{any::Any, panic::AssertUnwindSafe};

/// A component used by query-access fixtures.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
struct Position;

/// A component that marks one side of a disjoint-query fixture.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
struct Player;

/// A second component that forms another `Or` filter branch.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
struct Enemy;

/// A user-defined component that can be registered as an entity-disabling filter.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
struct Dormant;

/// A component also accessed through Bevy's non-send resource parameter.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
struct LocalValue;

/// Two alternative component-presence branches accepted by Bevy.
type TwoWayFilter = Or<(With<Player>, With<Enemy>)>;
/// Four alternative component-presence branches accepted by Bevy.
type FourWayFilter = Or<(TwoWayFilter, TwoWayFilter)>;
/// Eight alternative component-presence branches accepted by Bevy.
type EightWayFilter = Or<(FourWayFilter, FourWayFilter)>;
/// Sixteen alternative component-presence branches accepted by Bevy.
type SixteenWayFilter = Or<(EightWayFilter, EightWayFilter)>;
/// Thirty-two alternative component-presence branches accepted by Bevy.
type ThirtyTwoWayFilter = Or<(SixteenWayFilter, SixteenWayFilter)>;
/// Sixty-four alternative component-presence branches accepted by Bevy.
type SixtyFourWayFilter = Or<(ThirtyTwoWayFilter, ThirtyTwoWayFilter)>;
/// A conjunction whose DNF expansion exceeds this lint's supported bound.
type ProductAboveLimitFilter = (SixtyFourWayFilter, TwoWayFilter);

/// A type that is addressable as both a Bevy resource and a component.
#[derive(Resource, Reflect)]
struct SharedValue(usize);

/// Assert that `SystemState` initialization fails with Bevy's reported conflict code.
fn assert_panics_with_code<T>(result: Result<T, Box<dyn Any + Send>>, code: &str) {
    let payload = match result {
        Err(payload) => payload,
        Ok(_state) => panic!("expected SystemState initialization to panic with `{code}`"),
    };
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&'static str>().copied())
        .expect("Bevy conflict panics use string payloads");

    assert!(
        message.contains(code),
        "expected panic to contain `{code}`, got `{message}`"
    );
}

/// Bevy rejects overlapping mutable and shared access to one component during initialization.
#[test]
fn overlapping_queries_panic_with_b0001() {
    let mut world = World::new();
    let _position_entity = world.spawn(Position);

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _state = SystemState::<(Query<'_, '_, &mut Position>, Query<'_, '_, &Position>)>::new(
            &mut world,
        );
    }));

    assert_panics_with_code(result, "error[B0001]");
}

/// Bevy permits repeated shared access and mutable access to different component types.
#[test]
fn shared_and_distinct_component_queries_initialize() {
    // Repeated shared component reads are compatible during initialization.
    let mut world = World::new();

    let mut shared =
        SystemState::<(Query<'_, '_, &Position>, Query<'_, '_, &Position>)>::new(&mut world);
    let (_first, _second) = shared
        .get_mut(&mut world)
        .expect("shared query access is compatible");

    let mut distinct =
        SystemState::<(Query<'_, '_, &mut Position>, Query<'_, '_, &mut Enemy>)>::new(&mut world);
    let (_position, _enemy) = distinct
        .get_mut(&mut world)
        .expect("different component accesses are compatible");
}

/// Bevy permits mutable queries whose `With` and `Without` filters are complementary.
#[test]
fn complementary_with_and_without_queries_initialize_and_match_disjoint_entities() {
    // Insert one entity for each side of the complementary filter pair.
    let mut world = World::new();
    let _player_entity = world.spawn((Position, Player));
    let _unmarked_entity = world.spawn(Position);

    // Initialize both queries together, then verify each sees only its side.
    let mut state = SystemState::<(
        Query<'_, '_, &mut Position, With<Player>>,
        Query<'_, '_, &mut Position, Without<Player>>,
    )>::new(&mut world);
    let (mut players, mut non_players) = state
        .get_mut(&mut world)
        .expect("disjoint query parameters are valid");

    assert_eq!(players.iter_mut().count(), 1);
    assert_eq!(non_players.iter_mut().count(), 1);
}

/// Bevy rejects an `Or` query when any branch overlaps another query's filter.
#[test]
fn an_overlapping_or_branch_panics_with_b0001() {
    let mut world = World::new();
    let _player_entity = world.spawn((Position, Player));

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _state = SystemState::<(
            Query<'_, '_, &mut Position, Or<(With<Player>, With<Enemy>)>>,
            Query<'_, '_, &Position, With<Player>>,
        )>::new(&mut world);
    }));

    assert_panics_with_code(result, "error[B0001]");
}

/// An `Or` filter with no alternatives matches no entities in Bevy.
#[test]
fn zero_branch_or_filter_matches_no_entities() {
    // An empty Or has no matching branch but leaves the broad query valid.
    let mut world = World::new();
    let _position_entity = world.spawn(Position);
    // Initialize both parameters before checking how many entities they match.
    let mut state = SystemState::<(
        Query<'_, '_, &mut Position, Or<()>>,
        Query<'_, '_, &Position>,
    )>::new(&mut world);
    let (mut no_alternatives, broad) = state
        .get_mut(&mut world)
        .expect("zero-branch Or initializes");

    assert_eq!(no_alternatives.iter_mut().count(), 0);
    assert_eq!(broad.iter().count(), 1);
}

/// An empty `Or` filter does not hide conflicting component access within query data.
#[test]
fn zero_branch_or_filter_does_not_hide_internal_query_conflict() {
    // The filter controls matching entities, not duplicate access declarations.
    let mut world = World::new();
    let _position_entity = world.spawn(Position);
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        SystemState::<Query<'_, '_, (&mut Position, &Position), Or<()>>>::new(&mut world)
    }));
    // Require the internal-access failure before checking its diagnostic wording.
    let Err(payload) = result else {
        panic!("Bevy accepted conflicting component access within query data");
    };
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&'static str>().copied())
        .expect("Bevy query access panics use string payloads");

    assert!(
        message.contains("conflicts with a previous access in this query"),
        "unexpected panic: {message}"
    );
}

/// Bevy does not treat a contradiction within one query filter as proof of no overlap.
#[test]
fn contradictory_query_filter_does_not_disjoin_another_query() {
    let mut world = World::new();

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _state = SystemState::<(
            Query<'_, '_, &mut Position, (With<Player>, Without<Player>)>,
            Query<'_, '_, &Position>,
        )>::new(&mut world);
    }));

    assert_panics_with_code(result, "error[B0001]");
}

/// Bevy permits an `Or` query beside a query that excludes every branch marker.
#[test]
fn an_or_query_and_its_complement_initialize_and_match_disjoint_entities() {
    // Spawn entities covered by each branch and by neither branch.
    let mut world = World::new();
    let _player_entity = world.spawn((Position, Player));
    let _enemy_entity = world.spawn((Position, Enemy));
    let _unmarked_entity = world.spawn(Position);

    // The complementary tuple filters partition all three entities.
    let mut state = SystemState::<(
        Query<'_, '_, &mut Position, Or<(With<Player>, With<Enemy>)>>,
        Query<'_, '_, &mut Position, (Without<Player>, Without<Enemy>)>,
    )>::new(&mut world);
    let (mut marked, mut unmarked) = state
        .get_mut(&mut world)
        .expect("complementary Or filters are valid");

    assert_eq!(marked.iter_mut().count(), 2);
    assert_eq!(unmarked.iter_mut().count(), 1);
}

/// Bevy permits internally conflicting queries in a `ParamSet` because callers
/// access them in turn.
#[test]
fn param_set_allows_sequential_access_to_conflicting_queries() {
    // Initialize the ParamSet before requesting each member in sequence.
    let mut world = World::new();
    let _position_entity = world.spawn(Position);

    let mut state = SystemState::<
        ParamSet<'_, '_, (Query<'_, '_, &mut Position>, Query<'_, '_, &Position>)>,
    >::new(&mut world);
    let mut params = state
        .get_mut(&mut world)
        .expect("ParamSet allows sequential query access");

    // Accessing member zero before member one honors the ParamSet boundary.
    assert_eq!(params.p0().iter_mut().count(), 1);
    assert_eq!(params.p1().iter().count(), 1);
}

/// Bevy still rejects a `ParamSet` whose internal mutable access conflicts externally.
#[test]
fn param_set_access_conflicts_with_an_external_query() {
    let mut world = World::new();

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _state = SystemState::<(
            ParamSet<'_, '_, (Query<'_, '_, &mut Position>, Query<'_, '_, &Position>)>,
            Query<'_, '_, &Position>,
        )>::new(&mut world);
    }));

    assert_panics_with_code(result, "error[B0001]");
}

/// Conflicting queries in one tuple-valued `ParamSet` member remain simultaneous.
#[test]
fn one_tuple_param_set_member_with_conflicting_queries_panics_with_b0001() {
    let mut world = World::new();
    let _position_entity = world.spawn(Position);

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _state = SystemState::<
            ParamSet<'_, '_, ((Query<'_, '_, &mut Position>, Query<'_, '_, &Position>),)>,
        >::new(&mut world);
    }));

    assert_panics_with_code(result, "error[B0001]");
}

/// Defaults exclude `Disabled`, but not `IsResource`, from query access.
#[test]
fn default_filters_do_not_exclude_resource_entities_from_queries() {
    // Inspect the world's installed filters before proving resource overlap.
    let mut world = World::new();
    world.insert_resource(SharedValue(1));

    let filters = world
        .get_resource::<DefaultQueryFilters>()
        .expect("World installs default query filters");
    let disabling_ids: Vec<_> = filters.disabling_ids().collect();
    let disabled_id = world
        .component_id::<Disabled>()
        .expect("World initializes Disabled as a component");
    let is_resource_id = world
        .component_id::<IsResource>()
        .expect("World registers IsResource during bootstrap");
    assert_eq!(disabling_ids, vec![disabled_id]);
    assert!(!disabling_ids.contains(&is_resource_id));

    // The defaults exclude Disabled but do not exclude resource entities.
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _state = SystemState::<(Query<'_, '_, &mut SharedValue>, ResMut<'_, SharedValue>)>::new(
            &mut world,
        );
    }));

    assert_panics_with_code(result, "error[B0002]");
}

/// Bevy rejects a query that overlaps a non-send resource component access.
#[test]
fn query_and_non_send_resource_conflict_with_b0002() {
    let mut world = World::new();

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _state =
            SystemState::<(Query<'_, '_, &mut LocalValue>, NonSendMut<'_, LocalValue>)>::new(
                &mut world,
            );
    }));

    assert_panics_with_code(result, "error[B0002]");
}

/// Bevy permits query and resource access when the resource marker is explicitly
/// excluded.
#[test]
fn excluding_is_resource_disjoins_query_and_resource_access() {
    // Insert a resource, then ensure the explicit marker filter excludes it.
    let mut world = World::new();
    world.insert_resource(SharedValue(1));

    let mut state = SystemState::<(
        Query<'_, '_, &mut SharedValue, Without<IsResource>>,
        ResMut<'_, SharedValue>,
    )>::new(&mut world);
    let (mut components, mut resource) = state
        .get_mut(&mut world)
        .expect("Without<IsResource> separates query and resource access");

    // Read and write through each disjoint access path.
    assert_eq!(components.iter_mut().count(), 0);
    resource.0 += 1;
    assert_eq!(resource.0, 2);
}

/// Bevy permits an explicit `Without<T>` filter beside `ResMut<T>`.
#[test]
fn excluding_resource_type_disjoins_query_and_resource_access() {
    // Insert the resource before initializing a query that excludes its entity type.
    let mut world = World::new();
    world.insert_resource(SharedValue(1));

    // The explicit exclusion lets the query and ResMut share one Rust type.
    let mut state = SystemState::<(
        Query<'_, '_, &mut SharedValue, Without<SharedValue>>,
        ResMut<'_, SharedValue>,
    )>::new(&mut world);
    let (mut components, resource) = state
        .get_mut(&mut world)
        .expect("Without<ResourceType> separates query and resource access");

    assert_eq!(components.iter_mut().count(), 0);
    assert_eq!(resource.0, 1);
}

/// Bevy permits a query beside a resource parameter when the component types differ.
#[test]
fn query_and_distinct_resource_access_initialize() {
    // The query component and resource types are intentionally different.
    let mut world = World::new();
    let _position_entity = world.spawn(Position);
    world.insert_resource(SharedValue(1));

    // Initialize both accesses together and confirm each remains usable.
    let mut state =
        SystemState::<(Query<'_, '_, &mut Position>, ResMut<'_, SharedValue>)>::new(&mut world);
    let (mut positions, mut resource) = state
        .get_mut(&mut world)
        .expect("different component types do not conflict");

    assert_eq!(positions.iter_mut().count(), 1);
    resource.0 += 1;
    assert_eq!(resource.0, 2);
}

/// Bevy permits query and resource access in separate `ParamSet` members.
#[test]
fn query_and_resource_param_set_members_are_sequential() {
    // Put same-type mutable accesses in separate ParamSet alternatives.
    let mut world = World::new();
    world.insert_resource(SharedValue(1));

    // Bevy initializes the set, then permits sequential member access.
    let mut state = SystemState::<
        ParamSet<'_, '_, (Query<'_, '_, &mut SharedValue>, ResMut<'_, SharedValue>)>,
    >::new(&mut world);
    let mut params = state
        .get_mut(&mut world)
        .expect("ParamSet defers conflicting query and resource access");

    assert_eq!(params.p0().iter_mut().count(), 1);
    params.p1().0 += 1;
    assert_eq!(params.p1().0, 2);
}

/// Bevy accepts a `ParamSet<Vec<T>>` with its initially empty parameter list.
#[test]
fn empty_vec_param_set_initializes() {
    let mut world = World::new();
    let _state =
        SystemState::<ParamSet<'_, '_, Vec<Query<'_, '_, &mut Position>>>>::new(&mut world);
}

/// Bevy accepts filter conjunctions beyond the lint's analysis bound.
#[test]
fn large_conjunctive_or_filter_initializes() {
    // The second conjunction doubles a valid filter beyond the static expansion cap.
    let mut world = World::new();
    let _player = world.spawn((Position, Player));

    let mut state =
        SystemState::<Query<'_, '_, &Position, ProductAboveLimitFilter>>::new(&mut world);
    let position_query = state
        .get_mut(&mut world)
        .expect("large filter products remain valid Bevy filters");

    assert_eq!(position_query.iter().count(), 1);
}

/// A preceding non-send resource access can be disjoined by `Without<T>` on the query.
#[test]
fn preceding_non_send_resource_is_disjoined_by_query_type_filter() {
    let mut world = World::new();

    let _state = SystemState::<(
        NonSendMut<'_, LocalValue>,
        Query<'_, '_, &mut LocalValue, Without<LocalValue>>,
    )>::new(&mut world);
}

/// A query before `NonSendMut` is checked without considering its filter.
#[test]
fn query_before_non_send_resource_still_conflicts_despite_type_filter() {
    let mut world = World::new();
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        SystemState::<(
            Query<'_, '_, &mut LocalValue, Without<LocalValue>>,
            NonSendMut<'_, LocalValue>,
        )>::new(&mut world)
    }));

    assert_panics_with_code(result, "error[B0002]");
}

/// A query filter on `IsResource` does not separate a non-send resource access.
#[test]
fn excluding_is_resource_does_not_disjoin_non_send_resource_access() {
    let mut world = World::new();
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        SystemState::<(
            NonSendMut<'_, LocalValue>,
            Query<'_, '_, &mut LocalValue, Without<IsResource>>,
        )>::new(&mut world)
    }));

    assert_panics_with_code(result, "error[B0001]");
}

/// The default Disabled filter changes whether a query pair can overlap.
#[test]
fn changing_default_query_filters_changes_the_overlap_result() {
    // Default filters separate the Disabled branch from the broad query.
    let mut default_world = World::new();
    let _disabled_entity = default_world.spawn((Position, Disabled));
    let _enabled_entity = default_world.spawn(Position);

    let mut state = SystemState::<(
        Query<'_, '_, &mut Position, With<Disabled>>,
        Query<'_, '_, &Position>,
    )>::new(&mut default_world);
    // Both parameter accesses initialize under Bevy's default Disabled exclusion.
    let (mut disabled, enabled) = state
        .get_mut(&mut default_world)
        .expect("default filters exclude the Disabled query");
    assert_eq!(disabled.iter_mut().count(), 1);
    assert_eq!(enabled.iter().count(), 1);

    // Removing the defaults makes the same pair overlap during initialization.
    let mut unfiltered_world = World::new();
    let _disabled_entity = unfiltered_world.spawn((Position, Disabled));
    unfiltered_world.insert_resource(DefaultQueryFilters::empty());

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _state = SystemState::<(
            Query<'_, '_, &mut Position, With<Disabled>>,
            Query<'_, '_, &Position>,
        )>::new(&mut unfiltered_world);
    }));

    assert_panics_with_code(result, "error[B0001]");
}

/// Registering a custom disabling component can make query filters disjoint.
#[test]
fn custom_default_disabling_component_changes_query_overlap() {
    // Register a project-specific disabling component before query setup.
    let mut world = World::new();
    world.register_disabling_component::<Dormant>();
    let _dormant = world.spawn((Position, Dormant));
    let _active = world.spawn(Position);

    // The registered default exclusion disjoins the two mutable query accesses.
    let mut state = SystemState::<(
        Query<'_, '_, &mut Position, With<Dormant>>,
        Query<'_, '_, &Position>,
    )>::new(&mut world);
    let (mut dormant, active) = state
        .get_mut(&mut world)
        .expect("custom default filters disjoin these query accesses");

    assert_eq!(dormant.iter_mut().count(), 1);
    assert_eq!(active.iter().count(), 1);
}

/// Bevy 0.18's default `Disabled` filter disjoins shared component queries.
#[test]
fn bevy_018_default_disabled_filter_disjoins_query_access() {
    // Spawn one disabled and one enabled component entity in the old world.
    let mut world = old_bevy::ecs::world::World::new();
    let _disabled_entity = world.spawn((OldPosition, old_bevy::ecs::entity_disabling::Disabled));
    let _enabled_entity = world.spawn(OldPosition);

    // Initialize the old query pair under that version's default filter.
    let mut state = old_bevy::ecs::system::SystemState::<(
        old_bevy::ecs::system::Query<
            '_,
            '_,
            &'static mut OldPosition,
            old_bevy::ecs::query::With<old_bevy::ecs::entity_disabling::Disabled>,
        >,
        old_bevy::ecs::system::Query<'_, '_, &'static OldPosition>,
    )>::new(&mut world);
    let (mut disabled, enabled) = state.get_mut(&mut world);

    assert_eq!(disabled.iter_mut().count(), 1);
    assert_eq!(enabled.iter().count(), 1);
}
