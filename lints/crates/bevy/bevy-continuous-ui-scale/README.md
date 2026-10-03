# bevy-continuous-ui-scale

## What it does

Checks direct system registrations in Bevy's built-in repeating schedules for supported `UiScale` assignments derived from time, `glam::Vec3::distance`, or a local value derived from those sources. The warning marks a potentially varying UI scale.

## Why is this bad?

When an application renders text, changing `UiScale` can change effective raster sizes and may cause Bevy to request additional font-atlas keys. The lint cannot prove that the application contains text or that a new key and glyph will be rendered.

## Known problems

The lint recognizes only direct registrations under Bevy's built-in repeating schedules and a bounded set of resolved expression forms. It tracks local values through direct assignments and `+=`, `-=`, `*=`, and `/=`; a statically known zero multiplier clears the local value's taint. It recognizes Bevy's resolved `run_once` condition on each registration, including through schedule-configuration wrappers, tuple groups or members, and conjunctions of Bevy system conditions. It still warns when the same function also has a registration that may repeat. Unknown or custom conditions remain potentially repeating unless a Bevy system-condition conjunction contains the resolved built-in `run_once`. Same-named application functions and disjunctions remain potentially repeating.

The lint also does not analyze runtime stability, paused time, whether either vector in `Vec3::distance` changes, whether the app renders text, or whether text rendering requests a new atlas key. A stable distance can still be assigned repeatedly.

Constant-operand accumulation written directly to `UiScale.0`, such as `scale.0 += 0.1`, is outside this source-based analysis and is not reported. The lint deliberately excludes rounding calls such as `time.elapsed_secs().floor()`, even though that value may change as elapsed time grows. It excludes `Time<Fixed>::delta` as a stable timestep; an application that changes that timestep at runtime can vary without a warning. Finite choices, constant reassignments, startup systems, and transform scaling are not reported.

The lint excludes `Vec3::distance` when both operands resolve directly to constant paths. Other constant vector expressions can still warn. Supported constant factors preserve `f32` or `f64` rounding at each arithmetic operation. The evaluator skips external constant bodies, integer arithmetic, casts, remainder operations, and values beyond its 16-level recursion limit.

The lint does not simplify algebraic cancellation, so a tracked value such as `size -= size` can still warn. It does not analyze control-flow reachability, including constant `if` or `match` branches and empty loops. A source-based warning therefore does not prove that the write executes or that its result changes.

## Example

```rust
# use bevy_app::{App, Update};
# use bevy_ecs::prelude::{Res, ResMut};
# use bevy_time::Time;
# use bevy_ui::UiScale;
fn main() {
    let mut app = App::new();
    app.add_systems(Update, animate_ui_scale);
}

fn animate_ui_scale(mut scale: ResMut<UiScale>, time: Res<Time>) {
    scale.0 = 1.0 + time.elapsed_secs() * 0.1;
}
```

## Use instead

Set UI scale from startup configuration or discrete user choices. For a continuous visual change, scale a suitable UI subtree when its layout and interaction behavior remain correct.
