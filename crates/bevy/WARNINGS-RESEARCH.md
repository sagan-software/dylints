# Bevy warning research

The font-atlas warning is [B0005](https://bevy.org/learn/errors/b0005/).
The best first performance lint is continuous font-size animation.
The best first correctness lints are conflicting resource parameters and
supported query conflicts.
These candidates were absent from the 40 registered Bevy lints at the research baseline.

Research date: 2026-10-02. Implementation selection: 2026-10-03, recorded below.

## Evidence and scope

Bevy was cloned into `/home/sagan/Code/github.com/bevyengine/bevy`, following
the machine's `Code/github.com/<owner>/<repository>` convention.
The checkout is at `v0.19.1`, commit
`b56fc29d3016e641754765244b5ba3f9cc504671`.
The fetched main branch identifies itself as `0.20.0-dev`; it is not the
baseline for the proposals below.

The comparison used this repository's Bevy registration functions,
constituent lint documentation, and existing `PROPOSALS.md`.
Upstream inspection covered the numbered error pages, their source sites,
text atlas generation, text hierarchy warnings, asset loading, rendering
compatibility checks, and selected examples. This is targeted research,
not a complete audit of every upstream warning or third-party lint.

## The nameplate failure

The September 30 TGF history contains a matching note: nameplates keep one
font size and scale with a transform to avoid a glyph atlas per distance.
It explicitly names B0005. The trace is in
`/home/sagan/.codex/sessions/2026/09/30/rollout-2026-09-30T11-51-42-01a0f371-943e-7ea0-904f-0564faf1cd2a.jsonl`,
line 20. This establishes the earlier discussion, not independent crash or
release verification.

B0005 describes separate font atlases for each font and font size, unreclaimed
memory when interpolating `TextFont::font_size` or `UiScale::scale`, and
`Transform::scale` as the alternative. Its references to `TextSettings` are
historical API guidance. B0005 and those settings are absent from the inspected
0.19.1 text source.

The underlying cost still has current evidence. The
[0.19.1 font-size documentation](https://docs.rs/bevy/0.19.1/bevy/text/enum.FontSize.html)
says each font-handle and scaled-size combination generates a new atlas.
The [atlas key](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_text/src/font_atlas_set.rs)
includes font ID, size bits, smoothing, and hinting. The
[text pipeline](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_text/src/pipeline.rs)
creates atlas entries for new keys and rasterizes missing glyphs.
An assignment alone does not prove that a new key or atlas is created.

The [TextFont documentation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_text/src/text.rs)
describes scaling by the window scale factor and `UiScale`, followed by
rounding to pixels. Entity transforms and camera projection do not change
the raster font size. Therefore, a stable font size plus transform scaling
avoids introducing font-size keys for that animation. Transform scaling can
pixelate enlarged text. The
[text2d example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/2d/text2d.rs)
demonstrates scaling and explicitly mentions that visual tradeoff.

The current pipeline also emits unnumbered warnings for font sizes at or below
zero and above 1000 logical pixels. The upper check uses the evaluated logical
size before scaling; it warns without clamping. This is separate from B0005.

## Numbered error candidates

- [B0001](https://bevy.org/learn/errors/b0001/): conflicting component access
  between system queries. A semantic lint can detect supported query/filter
  combinations before system initialization panics.
- [B0002](https://bevy.org/learn/errors/b0002/): conflicting resource access,
  including resources stored as components. Direct duplicate resource access
  is a strong candidate. The existing `bevy-unfiltered-entity-access-query`
  flags broad access followed by fixed typed access; it does not validate
  resource/query conflicts.
- [B0003](https://bevy.org/learn/errors/b0003/): commands or world operations
  targeting nonexistent entities. Existing world entity and batch lints cover
  some panic-prone APIs. General deferred entity lifetime requires runtime
  state; a local use-after-despawn rule needs a separate proof of command order.
- [B0004](https://bevy.org/learn/errors/b0004/): missing hierarchy-inherited
  components on a parent. A local hierarchy lint is possible, provided it
  accounts for required components and later insertions.
- [B0005](https://bevy.org/learn/errors/b0005/): dynamic font-atlas growth.
  The page remains useful motivation, but its API names need version checks.
- [B0006](https://bevy.org/learn/errors/b0006/): software rendering.
  Adapter selection is runtime behavior. A source lint cannot establish
  whether the selected device uses hardware acceleration.

Main additionally contains B0007, B0008, and B0009 in system-parameter
initialization. They are not part of the 0.19.1 baseline. Any implementation
must resolve its supported Bevy version instead of assuming the website's
error list matches every release.

## Detection reliability

Reliability here means a diagnostic is correct whenever its stated static
preconditions hold. It does not mean every occurrence of the runtime error
can be found. A conservative lint can skip unsupported code and still be useful.

## Can B0001 through B0006 be detected reliably?

Use separate semantic lints with error-code links in their diagnostics.
Do not promise a complete static implementation of all six runtime checks.

- B0001 has high-confidence static cases. Bevy rejects incompatible declared
  accesses when initializing the system, even if the world currently has no
  entity matching both queries. Model supported query data, filter alternatives,
  default filters, nested parameters, and `ParamSet` before emitting a definite
  conflict. Custom query and parameter implementations need access analysis
  or exclusion. Different positive marker filters do not prove disjointness.
- B0002 has high-confidence static cases for conflicting resource parameters.
  Compare resolved instantiated types. Repeated shared access is valid;
  mutable access paired with another access conflicts outside `ParamSet`.
  Resource storage as components adds query/resource cases that cannot be
  handled by comparing resource parameters alone. Arbitrary custom access
  declarations remain outside an initial rule.
- B0003 cannot be fully predicted from ordinary source. An entity can disappear
  because another system, scene unload, network response, or runtime command
  removes it. A same-queue, same-entity mutation after an unconditional despawn
  is a narrower candidate when aliases and intervening commands are understood.
  The numbered page shows historical panic behavior; current command error
  handling must be checked separately. Do not flag an intentional ignored
  failure through `try_insert` as a definite bug.
- B0004 permits local construction checks, but absence in one bundle does not
  prove absence when Bevy validates the hierarchy. The validator deliberately
  checks again later because the parent may still be populated. Account for
  required components and later inserts. Unknown bundles, scenes, hooks, and
  escaping entities prevent a definite prediction.
- B0005 supports a performance warning about a recognized animation pattern.
  It does not permit a compile-time prediction of atlas count, GPU memory,
  or a crash. Actual rendered sizes, rounding, glyphs, font identity, scale
  factors, run conditions, and cache contents determine the cost.
- B0006 needs runtime device evidence. Identical source can select a hardware
  adapter on one machine and a software adapter on another. A startup check
  or playtest log assertion can enforce hardware rendering; a source lint
  cannot establish the selected adapter.

For B0001 and B0002, describe the consequence as an initialization conflict
when the function is used as a system. An unregistered helper does not itself
establish that an initialization panic will occur.

## Initial candidates

### 1. `bevy-continuous-font-size`

Detect assignment to resolved `TextFont::font_size` from continuously varying
inputs in a directly registered repeating system. Initial supported sources
should include elapsed time, interpolation, accumulated deltas, and
camera-distance arithmetic. Detect whole-component replacement and
`with_font_size` construction when the same data flow is visible.

Recommend a stable raster font size and entity transform scaling. Emit help,
not an automatic replacement: text layout, wrapping, and visual quality can
change. Do not flag startup construction, a constant reassignment, a finite
choice of sizes, or every mutable `TextFont` query. A plain nonconstant RHS
does not establish continuous animation. Opaque helpers remain an analysis gap.

Test time-based assignments, distance-based nameplates, and component
replacement. Keep constant sizes, finite match choices, startup setup,
transform animation, unrelated `font_size` fields, and explicit bounded
quantization clean. Add a runtime experiment that compares atlas keys or
texture bytes for font-size animation and transform animation. A UI diagnostic
alone does not prove memory growth or a crash.

### 2. `bevy-continuous-ui-scale`

Apply the same continuous-input analysis to `UiScale` writes. This scale
affects raster font sizes, so a global UI animation can affect many text
entities. B0005 and current `TextFont` docs support this mechanism.

Recommend scaling an appropriate visual subtree where that preserves layout.
Do not flag a user-selected accessibility scale, startup configuration, or
discrete zoom steps. Test elapsed-time animation and accumulated changes;
keep fixed initialization and finite user settings clean. The font-atlas
experiment should include UI scale separately from entity transform scale.

### 3. `bevy-conflicting-resource-params`

Resolve system parameter types and detect `ResMut<T>` paired with `Res<T>` or
another `ResMut<T>` for the same type. Include the equivalent `NonSend`
conflicts. The [system parameter implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ecs/src/system/system_param.rs)
and B0002 document the initialization panic.

Recommend one parameter or a correctly scoped `ParamSet`. Do not compare
parameters inside one `ParamSet` as simultaneous accesses. Custom derived
`SystemParam` requires recursive inspection; unknown access must remain
unsupported rather than assumed safe. Cross-check resource-as-component access
against the existing broad entity access rule to avoid duplicate diagnostics.

Test mutable/shared and mutable/mutable pairs, nested ordinary tuples,
type aliases, and non-send conflicts. Keep repeated shared access, different
resource types, and `ParamSet` clean. Verify Bevy initialization fails for each
triggering case and succeeds for each supported alternative.

### 4. `bevy-conflicting-query-params`

Resolve query component accesses and determine whether filters prove the
queries disjoint. B0001 supplies both the failure and supported alternatives.
`With<Player>` and `With<Enemy>` alone do not establish disjointness.
An entity can have both marker components.

Initially support direct component-reference query data and `With`, `Without`,
tuple, and `Or` filters. Evaluate filter combinations as alternatives; do not
flatten `Or` into one conjunction. Skip unknown custom query data and filters.
Respect `ParamSet` boundaries. Recommend an explicit disjoint filter or
`ParamSet`, without choosing game semantics for the user.

Test mutable/shared and mutable/mutable overlaps with no filters and with
different positive markers. Keep complementary exclusions, shared/shared
access, distinct component types, and `ParamSet` clean. Include an `Or` case
with one overlapping branch. Verify initialization outcomes against Bevy.
The existing component contention lint concerns different systems and does
not replace this same-system access check.

### 5. `bevy-incompatible-msaa`

Detect a locally established camera bundle containing enabled `Msaa` and one
of `DeferredPrepass`, `ScreenSpaceAmbientOcclusion`, or
`OrderIndependentTransparencySettings`.
The [deferred check](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_core_pipeline/src/core_3d/mod.rs)
warns and disables MSAA. The
[SSAO check](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_pbr/src/ssao/mod.rs)
logs an error and skips SSAO extraction. The
[OIT check](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_core_pipeline/src/oit/mod.rs)
panics. Preserve these distinct consequences in diagnostics.

Recommend `Msaa::Off`. The default is `Sample4`, so an omitted `Msaa` can
also be a problem. Infer that default only when required-component expansion
and subsequent local inserts are known. Unknown bundle members or later
configuration cannot justify a definite error.

Test each incompatible component with `Sample2`, `Sample4`, `Sample8`, and
known default construction. Keep `Msaa::Off`, ordinary forward cameras, and
different entities clean. Use the
[deferred example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/3d/deferred_rendering.rs)
and [SSAO example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/3d/ssao.rs)
as positive compatibility references.

### 6. `bevy-asset-source-after-asset-plugin`

Detect `register_asset_source` after installation of `AssetPlugin` or a known
`DefaultPlugins` group on the same app. Also detect `WebAssetPlugin` added
after `AssetPlugin` when plugin ordering is statically visible.
The [registration implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_asset/src/lib.rs)
logs the ordering error; the
[web plugin](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_asset/src/io/web.rs)
warns about the same prerequisite.

Recommend moving source registration before the asset plugin. Resolve plugin
identity and inspect supported group configuration. Do not assume a disabled
asset plugin is present. Arbitrary plugin bodies need interprocedural analysis.

Test direct app chains, ordered plugin tuples, both asset plugin forms, and
web plugin ordering. Keep early registration and explicitly disabled asset
plugins clean. The
[custom reader example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/asset/custom_asset_reader.rs)
adds its registering plugin before `DefaultPlugins`.

### 7. `bevy-load-builder-replaced-guard`

Detect two resolved `LoadBuilder::with_guard` calls on the same builder before
loading starts. The [method](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_asset/src/server/mod.rs)
warns that the second call drops the first guard before loading begins.
A guard's `Drop` may signal completion, so replacement can signal too early.

Recommend combining guards into one owned guard value or removing the
unintended replacement. Test a direct chain and a supported local move;
keep one guard, separate builders, and a tuple of guards clean.
The [synchronization example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/asset/multi_asset_sync.rs)
shows why guard lifetime matters. This is a narrow, low-cost first lint.

### 8. `bevy-invalid-constant-font-size`

Detect evaluated pixel sizes at or below zero and above 1000 logical pixels,
matching the current pipeline warnings. Preserve the distinction between
invisible text and expensive atlas generation. Relative or viewport units
cannot use the same raw-number threshold without knowing their evaluation
inputs. Intentional hide/show behavior makes the nonpositive rule a policy
warning rather than a universal bug.

Test zero, negative values, 1000, and a value just above 1000. Keep ordinary
pixel sizes and unknown viewport or rem evaluations clean. Do not invent a
NaN warning based on these comparisons: the inspected checks do not cover it.

### 9. `bevy-text-span-with-layout`

Detect `TextSpan` and `TextLayout` on the same locally known entity.
The [text hierarchy check](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_text/src/text.rs)
warns because `TextLayout` belongs on the root text entity.
Recommend configuring the ancestor `Text` or `Text2d` root.

Test a direct tuple bundle and supported local insertion. Keep root layouts
and span-only child bundles clean. A general orphan-span rule needs proof
that no later parenting occurs, so it should remain separate.

### 10. `bevy-missing-hierarchy-components`

Detect a closed local hierarchy where a child needs `InheritedVisibility` or
`GlobalTransform` and the parent lacks the corresponding component.
B0004 and the
[hierarchy validator](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_app/src/hierarchy.rs)
establish the contract. Required components matter: `Transform` supplies
`GlobalTransform`, and `Visibility` supplies visibility companions.

Test the official B0004 broken hierarchy and its corrected form. Keep
required-component insertion and supported later parent configuration clean.
Custom bundles, custom requirements, and scenes make this a prototype candidate
rather than a first implementation.

## Other warnings to retain as runtime checks

- Multiple `IsDefaultUiCamera` entities require an entity-count proof.
- A skybox image must have a cube texture view. The
  [skybox validator](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_core_pipeline/src/skybox/mod.rs)
  skips invalid images. Ordinary asset paths do not prove the loaded image's
  final view dimension; the official skybox example configures it after loading.
- Tiled textures above 1000 generated slices emit a performance warning.
  The [tiling implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_sprite/src/texture_slice/mod.rs)
  needs source rectangle, draw size, axes, and stretch value to predict count.
  A constant-input rule could work, but a small stretch value alone is insufficient.
- Duplicate asset loaders can be intentional overrides. A blanket duplicate
  registration lint would need policy beyond the warning itself.
- GPU feature and limit warnings, including SSAO and atmosphere plugin
  availability, need device evidence.

## Performance ideas requiring stronger proof

Repeated `Assets::add` for an invariant mesh or material in a repeating system
is worth prototyping. The
[implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_asset/src/assets.rs)
allocates a fresh asset handle for each call. Reusing an existing handle avoids
that recreation. Dynamic geometry and genuine entity creation are valid uses;
the source must prove invariant input and recurring execution before warning.
Strong-handle lifetime determines retention, so repetition alone does not prove
a memory leak. Measure asset count and allocation behavior in a prototype.

Calling `AssetServer::load` each frame is already parked in `PROPOSALS.md`
because loads are cached. `Changed<T>` scan costs and mutable engine-computed
components also have existing research or rules. They are not new findings.

## Implementation evidence requirements

Use resolved Bevy `DefId`s and instantiated types, including aliases and
re-exports. Do not recognize types from identifier spelling alone.
Do not label every nonconstant value dynamic or every registered system
unconditionally per-frame. Run conditions and schedules affect execution.
Emit no automatic fix when the alternative changes layout or app behavior.

Each implementation needs focused triggering and non-triggering UI tests,
the repository's new-lint gates, and version-specific runtime evidence for
the claimed engine consequence. This research did not execute a Bevy app,
measure atlas memory, or reproduce the reported crash.

## Expanded community search

The follow-up searched public Reddit posts, GitHub issues and discussions,
Bluesky posts, maintainer merge notes, and a community best-practices repository.
Searches also targeted X and Mastodon. Those searches did not produce a useful
additional technical source. Private Discord conversations were not inspected.
The results below are research proposals, not measured detection accuracy.

Community reports identify mistakes and motivations. Current engine source
controls the proposed behavior and version restrictions. Old API names are
translated only after checking the 0.19.1 implementation.
The [BevyFlock lint source tree](https://github.com/TheBevyFlock/bevy_cli/tree/main/bevy_lint/src/lints)
was also inspected for standalone counterparts to these new candidates.
The inspected tree and this repository's registered Bevy lint list contain
no dedicated rules for the additional patterns below.

### 11. `bevy-mouse-displacement-times-delta`

The [Reddit mouse-delta discussion](https://www.reddit.com/r/gamedev/comments/vdep5p/quick_tip_mousedelta_doesnt_need_deltatime/)
describes frame-rate-dependent sensitivity when displacement is multiplied
by delta time. Bevy's [camera orbit example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/camera/camera_orbit.rs)
and [first-person example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/camera/first_person_view_model.rs)
explicitly explain the same distinction.

Detect local data flow from resolved `MouseMotion::delta` or
`AccumulatedMouseMotion::delta` into camera rotation arithmetic multiplied by
`Time::delta_secs` or an equivalent duration in seconds. Mouse displacement
is already integrated over the frame. With sensitivity in radians per pixel,
displacement in pixels gives rotation in radians; multiplying by seconds
changes that expression's unit and makes the result frame-rate-dependent.

This is a high-confidence suspicious pattern within that use, not a universal
ban on multiplying mouse data by time. Exclude velocity estimation followed
by integration, intentionally time-dependent sensitivity, opaque helpers,
and keyboard or gamepad axes. Test equal total mouse displacement at different
frame durations; the supported direct-rotation path should give equal rotation.

### 12. `bevy-mouse-wheel-unit-ignored`

A [maintainer merge note](https://gist.github.com/alice-i-cecile/41b973b04e6949911dde5aca29d7560a)
led to [issue 24508](https://github.com/bevyengine/bevy/issues/24508) and
[PR 24457](https://github.com/bevyengine/bevy/pull/24457), which corrected
scroll-unit handling in a camera example. The
[current input types](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_input/src/mouse.rs)
distinguish `Line` from `Pixel` and document an approximate conversion factor.

Detect raw `MouseWheel::x/y` or `AccumulatedMouseScroll::delta` used as a zoom
magnitude while the associated unit is discarded on the supported local path.
Recommend unit-aware normalization or an explicit input adapter. Do not promise
one universal pixels-per-line conversion; the upstream issue explains its
platform dependence. Exclude sign-only step controls, known unit filtering,
logging, text/list scrolling with explicit unit semantics, and values that
escape into a helper. Test both unit variants and those exclusions.

### 13. `bevy-frame-button-edge-in-fixed-update`

[Discussion 9164](https://github.com/bevyengine/bevy/discussions/9164) asks
whether fixed-step input can lose or repeat button transitions. The
[current button input documentation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_input/src/button_input.rs)
defines `just_pressed` and `just_released` as frame-only states. The
[fixed-timestep example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/movement/physics_in_fixed_timestep.rs)
accumulates input before the fixed loop because it can run zero times in a frame.

Detect these edge queries on the standard engine-updated `Res<ButtonInput<T>>`
in a system directly registered to `FixedUpdate`. Zero fixed ticks can miss
an edge; multiple fixed ticks can observe the same frame edge repeatedly.
Recommend recording the edge before the fixed loop and consuming it under
the application's chosen policy. Do not invent a universal consume-once rule.

Treat this as a schedule warning with an explicit assumption that the default
input update behavior is active. Custom fixed-input resources, custom input
schedules, clearing guards, and wrappers can be valid. Keep `pressed` held-state
checks clean. Test zero, one, and multiple fixed ticks per frame. This is more
specific than the repository's blanket fixed-schedule restriction policy.

### 14. `bevy-completed-task-not-retired`

[Discussion 13072](https://github.com/bevyengine/bevy/discussions/13072)
contains a task panic caused by polling a task again after it yielded its result.
The [current async-compute example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/async_tasks/async_compute.rs)
removes the task component on completion. `check_ready` does not itself retire
the task; changing the polling helper alone does not fix repeated polling.

Prototype analysis of task components polled in a repeating system where the
ready branch retains the same task and the same eligibility for future polls.
Recommend removing the task component, taking it from an `Option`, or making
the completed state ineligible. Exclude replacement, entity despawn, state
changes that exclude future polls, and helper calls whose retirement effect
is unknown. This needs control-flow and ownership analysis; missing a visible
`remove` call alone is not proof.

Test a task that resolves once, run the system again, and establish that the
defective case repolls it. Keep pending polls, removal, replacement, consumed
`Option` state, and changed query eligibility clean. Account for deferred
removal before another invocation can occur.

### 15. `bevy-readiness-check-consumes-task`

The task discussions prompted a separate ownership check. This candidate is
an inference from the current APIs. Bevy's
[now_or_never implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_tasks/src/futures.rs)
consumes its future and cancels it if not ready. `check_ready` instead borrows
the future. The [Task contract](https://docs.rs/async-task/4.7.1/async_task/struct.Task.html)
says dropping a task cancels it.

Detect a resolved Bevy task passed by value to a one-poll helper on a path
that intends to retry pending work but discards ownership on `None`.
An immediately discarded spawned task is related, but standard `must_use`
warnings may already catch a plain expression; inspect that coverage first.
Do not flag intentional cancellation, `detach`, owned await, or borrowed polls.
Test a stalled task: after a pending borrowed check it must remain runnable;
after a pending consuming check it is cancelled. Keep this distinct from
polling a completed task again.

### 16. `bevy-redundant-change-notification`

[Issue 8472](https://github.com/bevyengine/bevy/issues/8472) and the
[Reddit change-detection discussion](https://www.reddit.com/r/bevy/comments/16wuvjz/)
show recurring confusion about change tracking. Current
[change-detection examples](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/ecs/change_detection.rs)
recommend `set_if_neq`. The
[asset migration guide](https://github.com/bevyengine/bevy/blob/v0.19.1/_release-content/migration-guides/asset-mut-change-detection.md)
explains that unnecessary material modification events can cost rendering work.

Prototype an opt-in performance rule for unconditional assignment of a value
that can remain equal, through `Mut`, `ResMut`, or `AssetMut`, in a repeating
system. Match equality guards and supported update helpers before warning.
Do not flag calling `Assets::get_mut` alone: the 0.19.1 guard tracks mutable
borrows instead of treating every acquisition as modification.

Equality does not capture every application's invalidation semantics. A user
can intentionally signal a change even when a value compares equal. Therefore,
this is a policy lint with help-only diagnostics, not a definite correctness
error. Test equal and unequal assignments and downstream change/event behavior.
Keep equality guards and intentional explicit invalidation separate.
Do not report `is_changed` on initial insertion as a bug; it is documented behavior.

### 17. `bevy-message-presence-without-consumption`

[Discussion 6595](https://github.com/bevyengine/bevy/discussions/6595) and
[issue 10877](https://github.com/bevyengine/bevy/issues/10877) concern reader
retention and consumption. Current
[MessageReader documentation](https://docs.rs/bevy/0.19.1/bevy/ecs/message/struct.MessageReader.html)
explicitly pairs a presence check with `clear` to prevent retriggering.
The [iterator implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ecs/src/message/iterators.rs)
advances the cursor as messages are consumed; discarding an iterator does
not acknowledge its unread contents.

An exact narrow check can report `let _ = reader.read()` and discarded lazy
adapter chains as ineffective consumption. Standard unused-iterator diagnostics
already catch some expression statements, so test the incremental coverage.
A broader warning for a side effect guarded by `!reader.is_empty()` without
any consuming operation needs policy and control-flow analysis. Repeated
observation can be intentional.

Test two consecutive checks against one message. Keep `read().count()`,
`for_each`, fully consumed loops, and `clear` clean. A partial `next` consumes
only one message and can be intentional; do not label it broken by default.
Do not flag every conditional reader: message loss can be acceptable, and
custom buffer maintenance changes the retention contract.

### 18. `bevy-invariant-asset-created-in-loop`

The [Reddit drawing report](https://www.reddit.com/r/bevy/comments/1ky48b2/how_to_update_a_single_mesh_instead_of_summoning/)
shows a laggy application creating many mesh assets. It gives the existing
asset-recreation proposal a concrete use case. The
[current Assets implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_asset/src/assets.rs)
allocates a new handle for every `add`.

A narrower first rule can find loop-invariant mesh/material construction
inside a loop and recommend creating one asset before the loop and cloning
its handle. Loop repetition is statically visible without inferring frame
frequency. Shared material identity changes later mutation behavior, so a
valid optimization needs proof that independent identities are unnecessary.
Keep per-entity randomized assets, later independent mutations, opaque
constructors, and genuinely changing geometry clean. Do not claim asset
creation alone proves retained memory growth. Measure asset count and verify
the same visible output for a supported prototype.

## Community reports rejected or deferred

- A [Bluesky debugging post](https://bsky.app/profile/embersarc.bsky.social/post/3m2yef2dzqc22)
  led to [renderer-cache issue 21526](https://github.com/bevyengine/bevy/issues/21526).
  The report documents engine cache retention under legal material/entity usage.
  This supports version-specific regression checks, not a permanent lint
  against multiple material types or `NotShadowCaster`.
- [Discussion 14589](https://github.com/bevyengine/bevy/discussions/14589)
  confused buffered events and observers. Current `Message` and `Event`
  traits distinguish those APIs. The old built-in window-resize observer
  misuse is now rejected by trait bounds. Do not duplicate the compiler.
- The [best-practices repository](https://github.com/tbillington/bevy_best_practices)
  recommends stable application IDs instead of `Entity` for persistence and
  networking. The [Entity documentation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ecs/src/entity/mod.rs)
  limits raw-bit identity to one application instance. However, scene snapshots,
  entity remapping, and replication protocols can legitimately serialize
  entity references. A blanket `Serialize` or `to_bits` lint would be wrong.
  Reconsider only with a known external sink and proven lack of remapping.
- One-shot timer completion and ignored repeating-timer multiplicity can be
  mistakes, but a callback's intended frequency is application policy.
  The [current Timer source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_time/src/timer.rs)
  says repeating `is_finished` and `just_finished` behave identically.
  Do not build a generic replacement rule from outdated timer advice.
- Missing system order, missing resource initialization, missed buffered
  messages, and omitted state cleanup require a wider app-registration model.
  Plugin bodies and runtime conditions can supply the missing behavior.
  Absence in one function is insufficient evidence.

## Revised priorities

Implement high-confidence resource and supported query conflict checks first.
Keep the nameplate/font-size rule as the first performance prototype.
Next evaluate mouse displacement, scroll units, and narrow message-consumption
checks. Prototype task retirement and frame-edge input checks with runtime
regressions. Keep change-notification and shared-asset rules opt-in until
application intent and false-positive limits are established.

For B0003, B0004, B0005, and B0006, retain runtime diagnostics or playtest
assertions alongside any source lints. Static coverage cannot replace those
checks. The implementation selection below records the rules implemented after this research.

## General Rust performance gaps

The performance follow-up compared the repository's lint documentation with
the current [Clippy catalog](https://rust-lang.github.io/rust-clippy/master/index.html),
the [Rust lint catalog](https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html),
and the [Dylint example catalog](https://trailofbits.github.io/dylint/examples/).
The comparison includes Clippy's opt-in groups. A disabled existing rule is
not a new lint opportunity. This is a targeted comparison, not proof that
no third-party Rust analyzer implements any of these patterns.

At the research baseline, the local `perf` group contained `boxed_future_return`, `expensive_as_method`,
`owned_input_field_clones`, and `ownership_at_boundaries`. Performance-related
rules also exist outside that group: `collect_return`, tracing formatting
rules, reqwest client recreation rules, Tokio blocking rules, and Bevy access
and component-contention rules. None documents the six dedicated checks below.

### Existing coverage to reuse

- Clippy's `regex_creation_in_loops` already detects literal regex compilation
  in loops. Do not implement another general rule for the same pattern.
- `needless_collect`, `iter_overeager_cloned`, `redundant_clone`, and
  `assigning_clones` cover several avoidable collection and clone patterns.
  `assigning_clones` already recommends reuse through `clone_from` or
  `clone_into`. It is not a new proposal.
- `format_push_string`, `format_collect`, and `to_string_in_format_args`
  cover several formatting allocations. Existing local tracing rules cover
  inline allocated field values.
- `stable_sort_primitive` and `unnecessary_sort_by` cover sorting choices and
  simpler comparison expressions. They do not document expensive-key caching.
- `manual_contains` replaces a slice equality search with `contains`.
  It does not change repeated linear membership searches into indexed lookup.
- `reserve_after_initialization` recommends a capacity constructor.
  It does not reuse an allocation across loop iterations.
- Rust's `unused_allocation` has narrower allocation-elimination checks.
  It is not a general detector of repeated allocation or algorithmic cost.

### 19. `vec-front-removal-in-loop`

Detect repeated resolved `Vec::remove(0)` on the same vector in a loop,
especially a loop that drains it. The
[Vec documentation](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.remove)
specifies element shifting and recommends `VecDeque::pop_front` for front
removal. Draining n elements this way can move O(n squared) elements.

Recommend consuming iteration when the vector is drained without refilling,
or a queue when interleaved enqueue/dequeue behavior is required. Keep
single removals, fixed tiny buffers, and legitimate contiguous-storage
requirements separate. Changing the collection type can affect callers and
layout; emit help without an automatic fix.

Test empty, singleton, repeated draining, refill, and early-exit cases.
Verify order and drop behavior. Measure element moves or runtime as n grows.
Related repeated middle-removal loops can be assessed for `retain` or
`extract_if`, but callback order and access to removed values need separate
proof. Do not suggest `swap_remove` when order matters.

### 20. `growing-vec-linear-membership`

Detect a vector built by repeated `if !result.contains(&item)` checks followed
by `result.push(item)` in an input loop. The
[slice contract](https://doc.rust-lang.org/std/primitive.slice.html#method.contains)
specifies linear membership lookup. With distinct inputs, the growing search
performs O(n squared) comparisons.

For suitable hashable keys, recommend retaining the ordered result vector and
using a separate membership set. A set alone would lose first-seen order.
Keep tiny bounded collections, custom equality, non-hashable values, and
deliberate allocation avoidance clean or outside the first rule. Hashing and
extra storage can cost more for small inputs. Do not promise a universal speedup.

Test first-seen order, duplicates, empty input, and constant bounds. Start
with integer or stable identity keys. Measure comparisons, runtime, and the
extra storage before choosing a default level or minimum-size policy.

### 21. `expensive-sort-key`

Detect `sort_by_key` or equivalent comparison closures that construct an
allocated key, such as `name.to_lowercase()`, for repeated comparison.
The [cached-key contract](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by_cached_key)
limits key evaluation to once per element and uses temporary storage.
This can remove repeated allocation and transformation work.

Recommend `sort_by_cached_key` for deterministic expensive keys. Keep direct
field access and cheap arithmetic clean; the documented tradeoff can favor
`sort_by_key` for those cases. Arbitrary callbacks can have side effects,
panic, or produce different results across calls. Caching changes invocation
count and order, so it cannot be an unconditional automatic fix.

Test an allocating deterministic key, a cheap key, equal keys, and a
side-effecting callback. Measure key calls and allocations. Preserve stable
equal-key ordering when recommending a stable cached sort.

### 22. `scratch-buffer-recreated-in-loop`

Detect a local `Vec<u8>` or `String` created, filled, read, and discarded on
each loop iteration without escaping. Recommend retaining its capacity and
clearing its contents between iterations. Empty constructors alone do not
allocate; the rule must identify growth or explicit allocated capacity.

This differs from eliminating an unnecessary collection: the buffer remains
necessary, but its allocation can be reused. Start with byte/string buffers
to avoid changing arbitrary element destructors. Escape analysis, early
returns, nested closures, and cleanup timing limit the supported pattern.
Retaining capacity also retains peak memory, which can be undesirable.

For a directly registered recurring Bevy system, a related rule can recommend
`Local<Vec<u8>>` or `Local<String>`. Bevy's
[Local source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ecs/src/system/system_param.rs)
defines persistence across system calls. Account for schedule and run
conditions; registration alone does not prove unconditional frame frequency.

Test empty output, changing input size, escaping buffers, and early exits.
Measure allocation count and peak retained bytes under representative sizes.

### 23. `shrink-then-regrow-in-loop`

Detect a persistent vector or string repeatedly filled, cleared, and shrunk
inside a loop before the next fill. This discards capacity intended for reuse
and can cause allocator work on later growth. A shrink request does not
guarantee an allocator actually shrinks, so warn about the capacity pattern
without claiming a fixed number of allocations.

Keep explicit memory-release phases, rare oversized requests, and documented
memory limits outside a default rule. A loop alone does not prove that
retaining memory is the application's preferred tradeoff.

Test ordinary growth, bounded reuse, conditional large-buffer release, and
loop exit. Measure allocation calls and retained bytes. The recommendation
must state the retention cost rather than presenting reuse as always better.

### 24. `eager-logging-work`

Detect a local string formatted before a tracing event and used only for
that event. The work occurs before the macro can decide whether the event
is enabled. Existing inline-field rules do not document this local data-flow
case. A general extension should also assess recognized expensive preparation
used only by disabled diagnostics.
The [tracing 0.1.44 macro source](https://docs.rs/tracing/0.1.44/src/tracing/macros.rs.html)
gates field evaluation, including its optional log fallback. Its
[enabled-check documentation](https://docs.rs/tracing/0.1.44/tracing/macro.enabled.html)
describes expensive preparation and limitations of separate metadata checks.

Recommend moving eligible preparation inside the enabled macro path or using
borrowed structured fields. Restrict the first rule to known formatting and
supported local use chains. Formatting callbacks may have side effects;
changing when they run is observable. Keep other consumers, explicit enabled
guards, and application-required formatting clean.

Test disabled and enabled events, other consumers, and explicit guards.
Count formatting calls and allocations, and preserve the enabled event's
field value. Account for log compatibility and metadata-sensitive filtering.
Emit help without an automatic guard insertion; a separate enabled check
does not guarantee identical filtering behavior.

### Performance priorities and exclusions

Prototype repeated front removal and expensive sort keys first. Their source
patterns and documented cost mechanisms are narrow. Next evaluate scratch
buffer reuse and growing-vector membership with allocation and scaling
measurements. Treat shrink/regrow and eager logging as conditional proposals.

For Bevy, retain the existing proposals for continuous font size, continuous
UI scale, invariant asset recreation, and redundant change notification.
A separate narrow check for known blocking file I/O or thread sleep in a
recurring synchronous system could complement the existing async rules.
Startup systems, intentional pacing, helper calls, and task boundaries need
exclusions before that rule can claim frame impact.

Do not add blanket warnings for every allocation, clone, nested loop,
dynamic dispatch, `Changed<T>` filter, component insert/remove, or hash map.
Their cost and valid alternatives depend on workload or semantics. Prefer
diagnostics that name the detected operation and its cost mechanism. A lint
can establish a pattern; representative measurements must establish a speedup.

No performance prototype, benchmark, allocation experiment, or lint UI test
ran during this research. Detection accuracy remains unmeasured.

## Implementation selection

On 2026-10-03, fourteen rules were implemented with `Warn` defaults. The Bevy
fixtures use the locked 0.19.0 API. Query fixtures also exercise Bevy 0.18.1 to
check the boundary before resources became components. Each rule resolves
supported APIs and types semantically and documents unsupported forms.

Implemented Bevy rules:

- [`bevy-continuous-font-size`](bevy-continuous-font-size): time- or distance-based raster-size writes in directly registered repeating systems.
- [`bevy-continuous-ui-scale`](bevy-continuous-ui-scale): supported time- or distance-based writes to `UiScale` in repeating systems.
- [`bevy-invalid-constant-font-size`](bevy-invalid-constant-font-size): supported constant pixel sizes of zero, below zero, or above 1000 logical pixels.
- [`bevy-conflicting-resource-params`](bevy-conflicting-resource-params): simultaneous supported resource access, including `ParamSet` member boundaries.
- [`bevy-conflicting-query-params`](bevy-conflicting-query-params): supported query conflicts, disjoint filters, and version-specific resource access.
- [`bevy-incompatible-msaa`](bevy-incompatible-msaa): explicit bundle combinations that conflict with deferred rendering, SSAO, or OIT.
- [`bevy-text-span-with-layout`](bevy-text-span-with-layout): direct bundles containing both `TextSpan` and `TextLayout`.
- [`bevy-asset-source-after-asset-plugin`](bevy-asset-source-after-asset-plugin): source registration after an explicit asset plugin on the same app.
- [`bevy-load-builder-replaced-guard`](bevy-load-builder-replaced-guard): a second guard replaces a guard on a supported asset-server load builder.
- [`bevy-mouse-displacement-times-delta`](bevy-mouse-displacement-times-delta): supported frame mouse displacement multiplied by a non-fixed frame delta.
- [`bevy-frame-button-edge-in-fixed-update`](bevy-frame-button-edge-in-fixed-update): frame button edges read in directly registered `FixedUpdate` systems.
- [`bevy-message-presence-without-consumption`](bevy-message-presence-without-consumption): supported `MessageReader` iterators discarded through a wildcard binding.

Implemented general Rust performance rules:

- [`vec_front_removal_in_loop`](../../perf/vec_front_removal_in_loop): repeated front removal from a persistent `Vec` in supported loops.
- [`expensive_sort_key`](../../perf/expensive_sort_key): supported allocating key closures passed to stable `sort_by_key`.

Runtime tests check atlas-key separation, system-initialization conflicts, asset
source activation, guard drops, fixed-update input behavior, unread message
cursors, vector tail movement, and sort-key call counts. Render tests establish
the component combinations without claiming GPU rendering or crash reproduction.
The atlas-key test does not establish that every write allocates an atlas or
that the earlier nameplate crash was caused by GPU memory exhaustion.

The remaining proposals stay deferred. Task retirement, readiness polling,
scroll-unit policy, change notifications, and shared asset creation require
more application intent or control-flow evidence. Scratch-buffer reuse,
shrinking, collection membership, and eager logging also require cost or
ownership evidence beyond the implemented patterns. Existing component-size,
field-contention, and change-filter rules already cover the narrower ECS
concerns. A component's field count alone does not establish a performance defect.

B0001 and B0002 coverage remains partial. The query rule assumes Bevy's built-in
default query filters and skips unsupported query data and custom system
parameters. Runtime hierarchy and entity-lifetime failures remain runtime
checks. Each README states the detection boundary and possible false positives.

The implementation review added regressions for floating-point rounding,
value-producing mouse expressions, and opaque mutable-reference arguments.
Warnings remain bounded by the documented static analysis; implicit mutable
method receivers and arbitrary helper behavior are outside that analysis.

The fixed-input rule recognizes tuple parameters but skips `Option`, `ParamSet`,
and custom parameter wrappers. The continuous-size rules do not simplify
algebraic cancellation or prove control-flow reachability; constant branches
and empty loops can still warn. The asset rules clear tracked facts for known
closure calls and opaque closure arguments, which can suppress later warnings.

The query rule models the built-in `Disabled` default filter in both Bevy 0.18
and 0.19. Resource-component checks remain specific to Bevy 0.19. The vector
front-removal rule skips resolved zero-sized elements; generic element layouts
that cannot be resolved can still warn.

Each implemented rule includes triggering and non-triggering UI cases. Runtime tests
check the engine contract or the operation cost when that contract can be
exercised without rendering. Coverage reports must state their selected files,
uncovered lines, and tool limitations; passing UI tests alone does not establish
complete branch coverage or a measured game-frame speedup.
