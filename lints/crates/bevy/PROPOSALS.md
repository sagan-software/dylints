# Bevy lint proposals

- Crate: `bevy` 0.19.0, selected because `cargo search bevy` reported it as
  the latest stable release on 2026-07-25.
- Repository: <https://github.com/bevyengine/bevy/tree/v0.19.0>
- Homepage and official guide: <https://bevy.org> and
  <https://bevy.org/learn/>
- Rustdoc: <https://docs.rs/bevy/0.19.0/bevy/>
- Upstream lint reference:
  <https://github.com/TheBevyFlock/bevy_cli/tree/6d8cb94bb8ec0535f63884e50830547f226118a0/bevy_lint/src/lints>

## Documentation read

The review covered the complete public surface exposed by the Bevy 0.19.0
rustdoc JSON, including 29,824 exported paths. The `bevy` facade rustdoc stores
most APIs as external re-exports. Therefore, the review also scanned all 59
official workspace crates for Bevy and 1,145 crate source files. It also scanned
431 examples, 137 release and migration documents, the current official Bevy
Book, and the quick start. The source scan inspected 10,157 documentation signal matches for terms
including `must`, `should`, `avoid`, `panic`, `deprecated`, `safety`,
`performance`, `instead`, and `do not`.

The upstream parity review covered every declared lint in `bevy_lint`, its
implementation, and its positive and negative UI fixtures. The upstream
checkout uses Bevy 0.18, so API names and evidence below receive updates for
Bevy 0.19.

Coverage gaps: none for public Bevy 0.19 Rust APIs and official written
documentation. Platform-specific runtime behavior did not run because the
proposed rules are compile-time API and type checks.

## Ranked proposals

### 1. `bevy-insert-message-resource`

- Misuse: initializes or inserts `Messages<M>` through `App::init_resource` or
  `App::insert_resource`.
- Why this matters: the message update system has no installation, so buffers can
  grow indefinitely.
- Evidence:
  [`Messages`](https://docs.rs/bevy/0.19.0/bevy/ecs/message/struct.Messages.html)
  and
  [`App::add_message`](https://docs.rs/bevy/0.19.0/bevy/app/struct.App.html#method.add_message).
- Detection: resolve the `App` method and instantiated resource type. Next,
  prove that the resource ADT is `bevy_ecs::message::Messages`.
- Suggested fix: call `App::add_message::<M>()`; help-only when the message type
  cannot support exact rendering exactly.
- False-positive risk: low; manual `Messages` maintenance remains possible but
  has explicit documentation as advanced and leak-prone.
- Suitability: do now.

### 2. `bevy-iter-current-update-messages`

- Misuse: calls `Messages::iter_current_update_messages`.
- Why this matters: messages outside the narrow update window can receive no check or
  consumed repeatedly.
- Evidence:
  [`Messages::iter_current_update_messages`](https://docs.rs/bevy/0.19.0/bevy/ecs/message/struct.Messages.html#method.iter_current_update_messages).
- Detection: resolve the exact method on the `Messages` receiver type.
- Suggested fix: use `MessageReader<M>` and `read`; help-only.
- False-positive risk: low because the rustdoc begins with an explicit warning.
- Suitability: do now.

### 3. `bevy-unit-in-bundle`

- Misuse: passes `()` directly or inside a tuple to a Bevy bundle-taking spawn
  or insert method.
- Why this matters: unit remains outside the lint scope rather than inserted, often hiding an
  accidental call to a mutating method that returned `()`.
- Evidence:
  [`Bundle`](https://docs.rs/bevy/0.19.0/bevy/ecs/bundle/trait.Bundle.html) and
  the upstream `unit_in_bundle` rule and fixtures.
- Detection: resolve known bundle methods in Bevy ECS and recursively inspect the
  instantiated bundle argument type for unit tuple members.
- Suggested fix: use `spawn_empty` for an all-unit spawn; otherwise remove or
  replace the unit expression.
- False-positive risk: low because unit has no part effect.
- Suitability: do now.

### 4. `bevy-world-entity`

- Misuse: calls `World::entity`.
- Why this matters: the method panics when the entity does not exist.
- Evidence:
  [`World::entity`](https://docs.rs/bevy/0.19.0/bevy/prelude/struct.World.html#method.entity).
- Detection: resolve the exact method on `bevy_ecs::world::World`.
- Suggested fix: use `get_entity` and handle the `Result`.
- False-positive risk: low; this is an opt-in strict reliability rule.
- Suitability: do now.

### 5. `bevy-world-entity-mut`

- Misuse: calls `World::entity_mut`.
- Why this matters: the method panics when the entity does not exist.
- Evidence:
  [`World::entity_mut`](https://docs.rs/bevy/0.19.0/bevy/prelude/struct.World.html#method.entity_mut).
- Detection: resolve the exact method on `World`.
- Suggested fix: use `get_entity_mut` and handle the `Result`.
- False-positive risk: low; this is an opt-in strict reliability rule.
- Suitability: do now.

### 6. `bevy-world-resource`

- Misuse: calls `World::resource`.
- Why this matters: the method panics when the resource is absent.
- Evidence:
  [`World::resource`](https://docs.rs/bevy/0.19.0/bevy/prelude/struct.World.html#method.resource).
- Detection: resolve the exact method on `World`.
- Suggested fix: use `get_resource` and handle `Option`.
- False-positive risk: low; this is an opt-in strict reliability rule.
- Suitability: do now.

### 7. `bevy-world-resource-ref`

- Misuse: calls `World::resource_ref`.
- Why this matters: the method panics when the resource is absent.
- Evidence:
  [`World::resource_ref`](https://docs.rs/bevy/0.19.0/bevy/prelude/struct.World.html#method.resource_ref).
- Detection: resolve the exact method on `World`.
- Suggested fix: use `get_resource_ref` and handle `Option`.
- False-positive risk: low; this is an opt-in strict reliability rule.
- Suitability: do now.

### 8. `bevy-world-resource-mut`

- Misuse: calls `World::resource_mut`.
- Why this matters: the method panics when the resource is absent.
- Evidence:
  [`World::resource_mut`](https://docs.rs/bevy/0.19.0/bevy/prelude/struct.World.html#method.resource_mut).
- Detection: resolve the exact method on `World`.
- Suggested fix: use `get_resource_mut` and handle `Option`.
- False-positive risk: low; this is an opt-in strict reliability rule.
- Suitability: do now.

### 9. `bevy-world-non-send`

- Misuse: calls `World::non_send`.
- Why this matters: the method panics when the value is absent and has
  same-thread requirements.
- Evidence:
  [`World::non_send`](https://docs.rs/bevy/0.19.0/bevy/prelude/struct.World.html#method.non_send).
- Detection: resolve the exact method on `World`.
- Suggested fix: use `get_non_send` and handle `Option`.
- False-positive risk: low; this is an opt-in strict reliability rule.
- Suitability: do now.

### 10. `bevy-world-non-send-mut`

- Misuse: calls `World::non_send_mut`.
- Why this matters: the method panics when the value is absent and has
  same-thread requirements.
- Evidence:
  [`World::non_send_mut`](https://docs.rs/bevy/0.19.0/bevy/prelude/struct.World.html#method.non_send_mut).
- Detection: resolve the exact method on `World`.
- Suggested fix: use `get_non_send_mut` and handle `Option`.
- False-positive risk: low; this is an opt-in strict reliability rule.
- Suitability: do now.

### 11. `bevy-world-run-schedule`

- Misuse: calls `World::run_schedule`.
- Why this matters: the method panics if the named schedule does not exist.
- Evidence:
  [`World::run_schedule`](https://docs.rs/bevy/0.19.0/bevy/prelude/struct.World.html#method.run_schedule).
- Detection: resolve the exact method on `World`.
- Suggested fix: use `try_run_schedule` and handle the `Result`.
- False-positive risk: low; this is an opt-in strict reliability rule.
- Suitability: do now.

### 12. `bevy-world-schedule-scope`

- Misuse: calls `World::schedule_scope`.
- Why this matters: the method panics if the named schedule does not exist.
- Evidence:
  [`World::schedule_scope`](https://docs.rs/bevy/0.19.0/bevy/prelude/struct.World.html#method.schedule_scope).
- Detection: resolve the exact method on `World`.
- Suggested fix: use `try_schedule_scope` and handle the `Result`.
- False-positive risk: low; this is an opt-in strict reliability rule.
- Suitability: do now.

### 13. `bevy-world-insert-batch`

- Misuse: calls `World::insert_batch`.
- Why this matters: the method panics if any target entity does not exist.
- Evidence:
  [`World::insert_batch`](https://docs.rs/bevy/0.19.0/bevy/prelude/struct.World.html#method.insert_batch).
- Detection: resolve the exact method on `World`.
- Suggested fix: use `try_insert_batch` and handle the error.
- False-positive risk: low; this is an opt-in strict reliability rule.
- Suitability: do now.

### 14. `bevy-world-insert-batch-if-new`

- Misuse: calls `World::insert_batch_if_new`.
- Why this matters: the method panics if any target entity does not exist.
- Evidence:
  [`World::insert_batch_if_new`](https://docs.rs/bevy/0.19.0/bevy/prelude/struct.World.html#method.insert_batch_if_new).
- Detection: resolve the exact method on `World`.
- Suggested fix: use `try_insert_batch_if_new` and handle the error.
- False-positive risk: low; this is an opt-in strict reliability rule.
- Suitability: do now.

### 15. `bevy-main-return-without-app-exit`

- Misuse: a unit-returning `main` discards the result of `App::run`.
- Why this matters: the process loses Bevy's requested exit status.
- Evidence:
  [`App::run`](https://docs.rs/bevy/0.19.0/bevy/app/struct.App.html#method.run)
  and the official `return_after_run` example.
- Detection: inspect the entrypoint signature and resolve discarded `App::run`
  calls in its body.
- Suggested fix: return `AppExit` and tail-return the `run` result; help-only.
- False-positive risk: low because non-unit entrypoints remain outside the lint scope.
- Suitability: do now.

### 16. `bevy-global-transform-mutation-query`

- Misuse: requests `&mut GlobalTransform` as query data.
- Why this matters: Bevy documents `GlobalTransform` as engine-managed and says
  users cannot mutate it directly.
- Evidence:
  [`GlobalTransform`](https://docs.rs/bevy/0.19.0/bevy/transform/components/struct.GlobalTransform.html).
- Detection: inspect resolved `Query` data and find a mutable reference to the
  exact `bevy_transform::GlobalTransform` ADT.
- Suggested fix: query and mutate `Transform`.
- False-positive risk: low; engine-internal crates remain outside the lint scope.
- Suitability: do now.

### 17. `bevy-inherited-visibility-mutation-query`

- Misuse: requests `&mut InheritedVisibility` as query data.
- Why this matters: Bevy computes this part from `Visibility`.
- Evidence:
  [`InheritedVisibility`](https://docs.rs/bevy/0.19.0/bevy/camera/visibility/struct.InheritedVisibility.html).
- Detection: inspect resolved `Query` data for the exact mutable part
  reference.
- Suggested fix: mutate `Visibility`.
- False-positive risk: low; the rustdoc explicitly says users must not change
  the part manually.
- Suitability: do now.

### 18. `bevy-children-mutation-query`

- Misuse: requests `&mut Children` as query data.
- Why this matters: direct target-list mutation can desynchronize Bevy's
  relationship pair.
- Evidence:
  [`Children`](https://docs.rs/bevy/0.19.0/bevy/ecs/hierarchy/struct.Children.html).
- Detection: inspect resolved `Query` data for the exact mutable part
  reference.
- Suggested fix: change `ChildOf` on source entities or use relationship
  commands.
- False-positive risk: low because the rustdoc explicitly forbids direct
  manipulation.
- Suitability: do now.

### 19. `bevy-camera-modification-in-fixed-update`

- Misuse: adds a system with mutable query data filtered by `With<Camera>` to
  `FixedUpdate`.
- Why this matters: camera motion can be stale or jitter because fixed updates
  do not run once per rendered frame.
- Evidence: the official
  [physics in fixed timestep example](https://bevy.org/examples/movement/physics-in-fixed-timestep/)
  and upstream lint documentation.
- Detection: resolve `App::add_systems`, prove the schedule label is
  `FixedUpdate`, resolve direct system functions, and inspect their `Query`
  parameters.
- Suggested fix: run visual camera work in `Update` or the appropriate
  `RunFixedMainLoop` set.
- False-positive risk: low but not zero; only direct function systems with an
  explicit `With<Camera>` filter receive checks.
- Suitability: do now.

### 20. `bevy-zst-query`

- Misuse: queries a direct shared or mutable reference to a zero-sized
  part as data.
- Why this matters: marker components carry no runtime data and work better
  as filters.
- Evidence:
  [`Query`](https://docs.rs/bevy/0.19.0/bevy/ecs/system/struct.Query.html)
  and the upstream `zst_query` rule.
- Detection: inspect only direct reference query-data leaves and use rustc
  layout information; exclude `Has`, `AnyOf`, and other query adapters.
- Suggested fix: move the part to `With<T>`.
- False-positive risk: lower than upstream because adapter ZSTs remain outside the lint scope.
- Suitability: do now.

### 21. `bevy-borrowed-reborrowable`

- Misuse: takes `&mut` to a Bevy proxy type that already supports cheap
  reborrowing, such as `Commands`, `Query`, or `ResMut`.
- Why this matters: nested mutable references obscure ownership and make helper
  APIs less composable.
- Evidence: each supported type's `reborrow` or `as_mut` rustdoc and the
  upstream lint documentation.
- Detection: inspect function signatures for a mutable reference to an exact
  supported Bevy proxy ADT; exclude `self` and outputs tied to that input
  lifetime.
- Suggested fix: accept the proxy by value and reborrow at the call site;
  help-only.
- False-positive risk: low after lifetime-linked returns remain outside the lint scope.
- Suitability: do now.

### 22. `bevy-duplicate-plugin-addition`

- Misuse: directly chains two `add_plugins` calls with the same definitely
  unique plugin type.
- Why this matters: Bevy rejects duplicate unique plugins.
- Evidence:
  [`Plugin`](https://docs.rs/bevy/0.19.0/bevy/app/trait.Plugin.html) and
  [`App::add_plugins`](https://docs.rs/bevy/0.19.0/bevy/app/struct.App.html#method.add_plugins).
- Detection: resolve adjacent `App::add_plugins` calls, compare argument types,
  and skip plugin implementations that override `is_unique`.
- Suggested fix: remove the duplicate registration.
- False-positive risk: low because non-unique and dynamically determined
  plugins remain outside the lint scope.
- Suitability: do now.

### 23. `bevy-time-elapsed-secs-cast-f64`

- Misuse: casts `Time::elapsed_secs()` from `f32` to `f64`.
- Why this matters: the cast cannot recover precision already lost by the
  monotonically increasing `f32` value.
- Evidence:
  [`Time::elapsed_secs`](https://docs.rs/bevy/0.19.0/bevy/time/struct.Time.html#method.elapsed_secs)
  and
  [`Time::elapsed_secs_f64`](https://docs.rs/bevy/0.19.0/bevy/time/struct.Time.html#method.elapsed_secs_f64).
- Detection: match an `f64` cast whose operand resolves to the exact
  `Time::elapsed_secs` method.
- Suggested fix: call `elapsed_secs_f64`; machine-applicable when the method
  span is directly editable.
- False-positive risk: very low.
- Suitability: do now.

### 24. `bevy-missing-reflect`

- Misuse: a local `Component`, `Resource`, `Message`, or `Event` does not
  have a `Reflect` implementation.
- Why this matters: editor, inspection, serialization, and tooling workflows
  cannot discover the type.
- Evidence:
  [Bevy reflection guide](https://bevy.org/learn/book/storing-data/reflection/)
  and the upstream rule.
- Detection: compare semantically resolved local trait-implementation sets.
- Suggested fix: derive or do `Reflect` and register the type.
- False-positive risk: project-policy dependent, so the lint is opt-in.
- Suitability: do now.

### 25. `bevy-missing-default-for-unit-component`

- Misuse: a unit `Component` lacks `Default`.
- Why this matters: default construction remains necessary by common required
  part and scene workflows.
- Evidence:
  [`Component`](https://docs.rs/bevy/latest/bevy/prelude/trait.Component.html),
  docs for required ECS parts. The upstream rule.
- Detection: intersect local unit structs implementing `Component` with local
  `Default` implementations.
- Suggested fix: derive `Default`.
- False-positive risk: project-policy dependent, so the lint is opt-in.
- Suitability: do now.

### 26. `bevy-missing-clone-for-unit-component`

- Misuse: a unit `Component` lacks `Clone`.
- Why this matters: cloning, scene, and template workflows become needlessly
  restricted for a trivially cloneable marker.
- Evidence: official part and scene-template documentation plus the
  upstream rule.
- Detection: intersect local unit `Component` types with `Clone`
  implementations.
- Suggested fix: derive `Clone`.
- False-positive risk: project-policy dependent, so the lint is opt-in.
- Suitability: do now.

### 27. `bevy-missing-copy-for-unit-component`

- Misuse: a unit `Component` lacks `Copy`.
- Why this matters: a marker has no owned data and is trivially copyable.
- Evidence: official part documentation plus the upstream rule.
- Detection: intersect local unit `Component` types with `Copy`
  implementations.
- Suggested fix: derive `Copy` and `Clone`.
- False-positive risk: project-policy dependent, so the lint is opt-in.
- Suitability: do now.

### 28. `bevy-unconventional-naming`

- Misuse: a local `Plugin` name lacks `Plugin`, or a local `SystemSet` name
  lacks `Systems`.
- Why this matters: the type's role is less obvious in app construction and
  schedule configuration.
- Evidence: the official Bevy source conventions and upstream rule.
- Detection: inspect semantically resolved local trait implementations and
  their defining identifiers.
- Suggested fix: rename the type; help-only because references must also move.
- False-positive risk: style-policy dependent, so the lint is opt-in.
- Suitability: do now.

### 29. `bevy-disallow-update-schedule`

- Misuse: registers a system in `Update` where project policy requires fixed
  simulation.
- Why this matters: variable frame time can make fixed-step simulation
  nondeterministic.
- Evidence:
  [official schedules guide](https://bevy.org/learn/book/the-game-loop/schedules/)
  and the upstream rule.
- Detection: resolve `App::add_systems` and the exact `Update` label type.
- Suggested fix: select the intended fixed schedule.
- False-positive risk: intentionally policy-specific and opt-in.
- Suitability: do now.

### 30. `bevy-disallow-fixed-update-schedule`

- Misuse: registers a system in `FixedUpdate` where project policy requires
  per-frame updates.
- Why this matters: visual or input work can run zero or multiple times per
  rendered frame.
- Evidence:
  [official schedules guide](https://bevy.org/learn/book/the-game-loop/schedules/)
  and the upstream rule.
- Detection: resolve `App::add_systems` and the exact `FixedUpdate` label type.
- Suggested fix: select the intended per-frame schedule.
- False-positive risk: intentionally policy-specific and opt-in.
- Suitability: do now.

### 31. `bevy-duplicate-dependencies`

- Misuse: the rustc crate graph contains more than one resolved `bevy` facade
  version.
- Why this matters: equal-looking Bevy types from different versions are
  incompatible and commonly enter through mismatched plugins.
- Evidence: the upstream rule, Cargo's resolver documentation, and Bevy's
  official plugin compatibility guidance.
- Detection: count distinct loaded crates whose original crate name is `bevy`;
  emit one crate-level diagnostic.
- Suggested fix: align Bevy and plugin versions, using `cargo tree -d` to find
  the source.
- False-positive risk: low; deliberately supporting two Bevy versions is rare.
- Suitability: do now.

## Parked ideas

- Mutable `ComputedNode`: the rustdoc documents legitimate post-layout
  mutation, so a blanket query rule would be noisy.
- Mutable `ViewVisibility`: custom visibility systems match explicit expectations
  to set it in `CheckVisibility`.
- `Changed<T>` and `Added<T>` query filters: their full-scan cost has documentation,
  but use is often correct and requires workload knowledge.
- `Handle::clone_weak`: whether a strong handle exists elsewhere requires
  non-local ownership analysis.
- Calling `AssetServer::load` in a system: Bevy caches loads, and frequency
  alone does not prove a performance defect.

## ECS design addendum: part granularity and query scope

This addendum targets code that treats a Bevy part as an object with all
of an entity's state. The rules are project-policy lints. Bevy permits large
parts and wide queries. However, its scheduler detects conflicts at part
type granularity, and its default table storage iterates one column per
part type. Splitting state improves cache locality and system parallelism
only when systems query the resulting components narrowly.

The local Bevy 0.19.0 example corpus provides a useful conservative baseline.
Across 359 Rust example files, a source scan found 376 local `Component`
structs. Only two had five named fields, and none had more than five. Across
807 direct `Query<...>` occurrences, no query fetched more than five part
references and no query requested more than four mutable part references.
The dedicated custom-query example reached eight `QueryData` fields.

The motivating nature-sim revision shows both failure stages. The former
`AgentBody` part had 12 top-level fields and embedded 15-field physiology
and 16-field metrics values. The current split stores those values as separate
parts, but the transitional `AgentQuery` flattens to 49 part
accesses, including 47 mutable accesses. Narrow systems in the same revision
show the intended end state, such as translation, rotation, gaze, interaction,
and cooldown systems with separate query tuples.

### 32. `bevy-readonly-system-access`

- Misuse: a system requests `Query<&mut T>`, `ResMut<T>`, or a mutable field in
  derived `QueryData`, but the system only reads that value.
- Why this matters: the mutable access conflicts with every other read or
  write of `T`, so otherwise compatible systems cannot run in parallel. A
  mutable proxy can also mark `T` changed when code dereferences it mutably.
- Evidence:
  [`MultiThreadedExecutor`](https://docs.rs/bevy/0.19.0/bevy/ecs/schedule/struct.MultiThreadedExecutor.html),
  [`QueryData`](https://docs.rs/bevy/0.19.0/bevy/ecs/query/trait.QueryData.html),
  and
  [`DetectChangesMut`](https://docs.rs/bevy/0.19.0/bevy/ecs/change_detection/trait.DetectChangesMut.html).
- Detection: resolve direct Bevy system parameters. Next, inspect all uses of
  the fetched binding. Report only when no mutable dereference, mutable method,
  mutable reborrow, escape, or generated query-item method can mutate it.
- Suggested fix: replace `&mut T` with `&T`, `ResMut<T>` with `Res<T>`, or use
  the generated read-only `QueryData` type. The direct replacements can be
  machine-applicable.
- False-positive risk: low after escaped values and opaque method calls are
  excluded.
- Suitability: do now.

### 33. `bevy-unfiltered-entity-access-query`

- Misuse: a system uses `Query<EntityRef>` or `Query<EntityMut>` and
  accesses a fixed, statically known set of part types.
- Why this matters: Bevy registers `EntityRef` as read access to every
  part and `EntityMut` as write access to every part. `EntityMut`
  therefore conflicts with every part-accessing system.
- Evidence: the `WorldQuery` implementations for
  [`EntityRef`](https://docs.rs/bevy/0.19.0/bevy/ecs/world/struct.EntityRef.html)
  and
  [`EntityMut`](https://docs.rs/bevy/0.19.0/bevy/ecs/world/struct.EntityMut.html)
  call `read_all` and `write_all`; Bevy provides
  [`FilteredEntityRef`](https://docs.rs/bevy/0.19.0/bevy/ecs/world/struct.FilteredEntityRef.html)
  and
  [`FilteredEntityMut`](https://docs.rs/bevy/0.19.0/bevy/ecs/world/struct.FilteredEntityMut.html)
  for bounded dynamic access.
- Detection: resolve the exact query-data type and collect part type
  arguments passed to `get`, `get_mut`, and related accessors. Skip dynamic
  part IDs, reflection, serialization, and values that escape.
- Suggested fix: use an explicit query tuple for a fixed set. Use a filtered
  entity query when the part set forms dynamically. Help-only.
- False-positive risk: low because truly dynamic access remains outside the lint scope.
- Suitability: do now.

### 34. `bevy-large-component`

- Misuse: a named-field struct implements `Component`, has at least eight
  fields, and has a computed layout larger than 64 bytes.
- Why this matters: the default table stores the full value in one part
  column. A system that needs one field still fetches and conflicts on the
  whole part. Splitting independently accessed state lets queries touch
  fewer columns and lets systems borrow different part types.
- Evidence:
  [`Component`](https://docs.rs/bevy/latest/bevy/prelude/trait.Component.html)
  documents table storage as the default query-iteration layout, and Bevy's
  scheduler computes conflicts from part access. The official example
  baseline above has no part with more than five named fields.
- Detection: resolve the `Component` implementation semantically, count named
  fields, and query rustc's type layout. Exclude bundles, resources, one-field
  wrappers, arrays, enums, foreign types, and types whose layout is unknown.
- Suggested fix: split fields by lifecycle and system access. Keep a `Bundle`
  as the spawn-time recipe. Help-only because the correct grouping is domain
  specific.
- False-positive risk: medium. A cohesive configuration or value object can be
  valid, so the thresholds must be configurable and the lint must remain
  opt-in.
- Suitability: needs prototype.

### 35. `bevy-wide-query-access`

- Misuse: a direct query fetches more than five part references. A
  derived `QueryData` flattens to more than eight part fetches. Either
  form requests more than four mutable part types.
- Why this matters: every fetched part adds matching and fetch work.
  Every mutable part also expands the system's scheduler conflict set.
  Replacing one large part with many small parts does not improve
  access scope when a catch-all query immediately recombines them.
- Evidence:
  [`QueryData`](https://docs.rs/bevy/0.19.0/bevy/ecs/query/trait.QueryData.html)
  states that tuples and derived queries compose all member accesses. Bevy's
  multi-threaded executor runs only non-conflicting systems in parallel. The
  thresholds sit immediately above the official example maxima.
- Detection: semantically flatten tuples and nested derived `QueryData` into
  unique part read and write leaves. Do not count `Entity`, `With`,
  `Without`, `Has`, or other leaves that do no part data access.
- Suggested fix: define one query per system behavior. Keep wide projection
  queries confined to explicit snapshot or serialization boundaries.
  Help-only.
- False-positive risk: medium because snapshot, editor, and serialization
  systems can legitimately project many values.
- Suitability: needs prototype.

### 36. `bevy-partially-used-query-data`

- Misuse: a function instantiates a derived `QueryData` with more than eight
  flattened part fetches but reads or writes at most half of them.
- Why this matters: the unused fields still participate in query matching,
  fetching, and system access registration. Mutable unused fields can prevent
  parallel execution with systems that use those components.
- Evidence: the `QueryData` contract registers and fetches each declared field;
  its read-only generated type changes mutability but does not remove unused
  fields.
- Detection: map generated query-item fields back to the source `QueryData`,
  then collect field uses within the function. need at least four unused
  part leaves. Skip items passed to opaque functions, stored, returned,
  destructured with `..`, or used through an item method.
- Suggested fix: replace the catch-all query with a local tuple or a narrower
  behavior-specific `QueryData`. Help-only.
- False-positive risk: low for fully local item use after escape cases are
  excluded.
- Suitability: needs prototype.

### 37. `bevy-component-field-contention`

- Misuse: two scheduled systems ask for `&mut T` for the same multi-field
  part, but each system accesses a disjoint set of named fields.
- Why this matters: Bevy sees one part-level write conflict even though
  the domain state is independent. The systems cannot run together until the
  fields move to different part types.
- Evidence: reporting for schedule conflicts identifies conflicting part IDs,
  and the multi-threaded executor runs only systems with compatible part
  access sets.
- Detection: resolve functions passed directly to `add_systems`, collect
  field reads and writes for each `&mut T` query leaf, and compare systems in
  the same schedule. Report only disjoint, direct field access. Skip method
  calls, whole-value borrows, destructuring with `..`, ordered systems, and
  systems connected by an explicit dependency.
- Suggested fix: split `T` along the reported field-access clusters and query
  only the new part used by each system. Help-only.
- False-positive risk: low for the reported mechanism, but registration and
  ordering analysis make implementation complex.
- Suitability: needs prototype.

### 38. `bevy-presence-only-query-fetch`

- Misuse: a query fetches `&T` but only counts matching entities or ignores
  the value, or fetches `Option<&T>` and only tests `is_some` or `is_none`.
- Why this matters: the query requests part data when it needs only
  archetype membership. `With<T>` performs no part data access, and
  `Has<T>` returns membership without accessing `T`.
- Evidence:
  [`With<T>`](https://docs.rs/bevy/0.19.0/bevy/ecs/query/struct.With.html)
  performs no ECS data access, while
  [`Has<T>`](https://docs.rs/bevy/0.19.0/bevy/ecs/query/struct.Has.html)
  has documentation for callers that do not care about the part value.
- Detection: track each direct query tuple binding within the function. Report
  only unused bindings, iterator `count` or emptiness checks, and
  `Option<&T>` bindings used exclusively for presence tests.
- Suggested fix: move required presence to `With<T>` or replace optional data
  with `Has<T>`. Simple direct query rewrites can be machine-applicable.
- False-positive risk: low after values that escape or reach an opaque call are
  excluded.
- Suitability: do now.

### 39. `bevy-narrow-exclusive-system`

- Misuse: a function registered as a system takes `&mut World` but accesses
  only a fixed set of components and resources expressible as normal system
  parameters.
- Why this matters: `&mut World` makes the system exclusive. The executor must
  run it without any other system, even when its actual data access is narrow.
- Evidence: Bevy distinguishes exclusive systems in its executor, while the
  multi-threaded executor runs non-conflicting normal systems in parallel.
- Detection: resolve functions passed directly to `add_systems`. Next, collect
  calls to typed `World` part, resource, query, and command methods. Skip
  dynamic IDs, schedule mutation, entity allocation, reflection, world
  serialization, nested schedule execution, and escaping world references.
- Suggested fix: express the collected access as `Query`, `Res`, `ResMut`, and
  `Commands` parameters. Help-only.
- False-positive risk: low for a closed supported method set, but converting
  control flow and temporary borrows requires care.
- Suitability: needs prototype.

### 40. `bevy-large-component-change-filter`

- Misuse: `Changed<T>` or `Ref<T>::is_changed` observes a part that meets
  the `bevy-large-component` threshold. The consuming system reads only
  a subset of its fields.
- Why this matters: Bevy tracks change at part granularity. A write to an
  unrelated field wakes every consumer of `Changed<T>`, which hides the
  lifecycle boundaries that smaller components would expose.
- Evidence:
  [`Changed<T>`](https://docs.rs/bevy/0.19.0/bevy/ecs/query/struct.Changed.html)
  and [`Ref<T>`](https://docs.rs/bevy/0.19.0/bevy/ecs/change_detection/struct.Ref.html)
  expose part-level change ticks rather than field-level ticks.
- Detection: reuse the predicate for large components and local field-use analysis.
  Report only when fewer than half of the named fields receive reads and the value
  does not escape.
- Suggested fix: move the observed field group into its own part and
  filter on that part. Help-only.
- False-positive risk: medium because a cohesive aggregate can intentionally
  use one invalidation boundary.
- Suitability: needs prototype.

## ECS design ideas to park

- Flag every part above a byte-size threshold. Large arrays and cohesive
  math values can be valid components; field count and access evidence are
  needed.
- need one field per part. Official examples contain cohesive
  multi-field components, so this would enforce ceremony rather than Bevy
  design.
- need `SparseSet` for every marker or large part. Bevy documents
  sparse storage for frequent insertion and removal, not for size or marker
  status alone.
- Flag every part inserted and removed in source. Source presence does
  not prove runtime frequency. Therefore, it cannot select table versus sparse storage
  reliably.
- Flag every `Changed<T>` query. Bevy documents its scan cost, but change
  filtering is often the correct behavior and needs workload evidence.
