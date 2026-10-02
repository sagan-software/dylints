# bevy

Bevy 0.19 lints backed by the engine rustdocs, official Bevy guides, release and migration notes,
examples, and the `TheBevyFlock/bevy_cli` lint suite.

See [PROPOSALS.md](PROPOSALS.md) for the research coverage, ranked rules, detection limits, and
primary evidence.

## Lints

- [`bevy-borrowed-reborrowable`](bevy-borrowed-reborrowable)
- [`bevy-camera-modification-in-fixed-update`](bevy-camera-modification-in-fixed-update)
- [`bevy-children-mutation-query`](bevy-children-mutation-query)
- [`bevy-component-field-contention`](bevy-component-field-contention)
- [`bevy-disallow-fixed-update-schedule`](bevy-disallow-fixed-update-schedule)
- [`bevy-disallow-update-schedule`](bevy-disallow-update-schedule)
- [`bevy-duplicate-dependencies`](bevy-duplicate-dependencies)
- [`bevy-duplicate-plugin-addition`](bevy-duplicate-plugin-addition)
- [`bevy-global-transform-mutation-query`](bevy-global-transform-mutation-query)
- [`bevy-inherited-visibility-mutation-query`](bevy-inherited-visibility-mutation-query)
- [`bevy-insert-message-resource`](bevy-insert-message-resource)
- [`bevy-iter-current-update-messages`](bevy-iter-current-update-messages)
- [`bevy-large-component`](bevy-large-component)
- [`bevy-large-component-change-filter`](bevy-large-component-change-filter)
- [`bevy-main-return-without-app-exit`](bevy-main-return-without-app-exit)
- [`bevy-missing-clone-for-unit-component`](bevy-missing-clone-for-unit-component)
- [`bevy-missing-copy-for-unit-component`](bevy-missing-copy-for-unit-component)
- [`bevy-missing-default-for-unit-component`](bevy-missing-default-for-unit-component)
- [`bevy-missing-reflect`](bevy-missing-reflect)
- [`bevy-narrow-exclusive-system`](bevy-narrow-exclusive-system)
- [`bevy-partially-used-query-data`](bevy-partially-used-query-data)
- [`bevy-presence-only-query-fetch`](bevy-presence-only-query-fetch)
- [`bevy-readonly-system-access`](bevy-readonly-system-access)
- [`bevy-time-elapsed-secs-cast-f64`](bevy-time-elapsed-secs-cast-f64)
- [`bevy-unconventional-naming`](bevy-unconventional-naming)
- [`bevy-unfiltered-entity-access-query`](bevy-unfiltered-entity-access-query)
- [`bevy-unit-in-bundle`](bevy-unit-in-bundle)
- [`bevy-wide-query-access`](bevy-wide-query-access)
- [`bevy-world-entity`](bevy-world-entity)
- [`bevy-world-entity-mut`](bevy-world-entity-mut)
- [`bevy-world-insert-batch`](bevy-world-insert-batch)
- [`bevy-world-insert-batch-if-new`](bevy-world-insert-batch-if-new)
- [`bevy-world-non-send`](bevy-world-non-send)
- [`bevy-world-non-send-mut`](bevy-world-non-send-mut)
- [`bevy-world-resource`](bevy-world-resource)
- [`bevy-world-resource-mut`](bevy-world-resource-mut)
- [`bevy-world-resource-ref`](bevy-world-resource-ref)
- [`bevy-world-run-schedule`](bevy-world-run-schedule)
- [`bevy-world-schedule-scope`](bevy-world-schedule-scope)
- [`bevy-zst-query`](bevy-zst-query)
