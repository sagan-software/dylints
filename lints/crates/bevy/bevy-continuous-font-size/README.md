# bevy-continuous-font-size

## What it does

Checks direct system registrations in Bevy's built-in repeating schedules for supported assignments that derive `TextFont::font_size` from time, `glam::Vec3::distance`, or a local value derived from those sources. The warning marks a potentially varying raster size. Bevy documents the related atlas-growth concern as [B0005](https://bevy.org/learn/errors/b0005/).

## Why is this bad?

The effective font size contributes to the font-atlas key. When rendering needs a new key and glyph, Bevy may rasterize another atlas entry. An assignment alone does not prove that Bevy creates an entry, allocates a particular amount of memory, or crashes.

## Known problems

The lint recognizes only direct registrations under Bevy's built-in repeating schedules and a bounded set of resolved expression forms. It tracks local values through direct assignments and `+=`, `-=`, `*=`, and `/=`; a statically known zero multiplier clears the local value's taint. It recognizes Bevy's resolved `run_once` condition on each registration, including through schedule-configuration wrappers, tuple groups or members, and conjunctions of Bevy system conditions. It still warns when the same function also has a registration that may repeat. Unknown or custom conditions remain potentially repeating unless a Bevy system-condition conjunction contains the resolved built-in `run_once`. Same-named application functions and disjunctions remain potentially repeating.

The lint also does not analyze runtime stability, paused time, whether either vector in `Vec3::distance` changes, or whether text rendering requests a new atlas key. A stable distance can still be assigned repeatedly.

The lint deliberately excludes rounding calls such as `time.elapsed_secs().floor()`, even though that value may change as elapsed time grows. It excludes `Time<Fixed>::delta` as a stable timestep; an application that changes that timestep at runtime can vary without a warning. Finite choices, constant reassignments, startup systems, and transform scaling are not reported.

The lint excludes `Vec3::distance` when both operands resolve directly to constant paths. Other constant vector expressions can still warn. Supported constant factors preserve `f32` or `f64` rounding at each arithmetic operation. The evaluator skips external constant bodies, integer arithmetic, casts, remainder operations, and values beyond its 16-level recursion limit.

The lint does not simplify algebraic cancellation, so a tracked value such as `size -= size` can still warn. It does not analyze control-flow reachability, including constant `if` or `match` branches and empty loops. A source-based warning therefore does not prove that the write executes or that its result changes.

## Example

```rust
# use bevy_app::{App, Update};
# use bevy_ecs::prelude::{Query, Res};
# use bevy_text::{FontSize, TextFont};
# use bevy_time::Time;
fn main() {
    let mut app = App::new();
    app.add_systems(Update, animate_nameplates);
}

fn animate_nameplates(mut fonts: Query<&mut TextFont>, time: Res<Time>) {
    for mut font in &mut fonts {
        font.font_size = FontSize::Px(20.0 + time.elapsed_secs() * 0.2);
    }
}
```

## Use instead

Keep the raster size stable and animate the entity's transform scale when layout, wrapping, and visual quality allow it. Scaling can make enlarged text look pixelated.
