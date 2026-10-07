# bevy-invalid-constant-font-size

## What it does

Checks statically evaluable `FontSize::Px` values at or below zero and values above 1000 logical pixels, matching the Bevy 0.19 text-pipeline warnings.

## Why is this bad?

Nonpositive pixel sizes display no text. Sizes above 1000 logical pixels trigger a Bevy warning because they can use excessive font-atlas memory. Bevy checks this threshold before UI scaling.

## Known problems

The lint checks statically evaluable `f32` literals and local `f32` constant paths. It evaluates unary negation, empty blocks, and up to 16 recursive levels of `+`, `-`, `*`, or `/` when each expression resolves to `f32`. It skips `%`, casts, arithmetic performed as integers or `f64`, runtime expressions, viewport units, and rem units. It skips trait-associated constants, including defaults, because a generic type can override a default; local inherent associated constants with bodies are checked. A nonpositive size can be intentional, so use a visibility state when the intent is to hide text.

## Example

```rust
# use bevy::prelude::*;
# use bevy::text::FontSize;
fn main() {
    let font_size = FontSize::Px(1001.0);
    let _ = font_size;
}
```

## Use instead

Use a positive size such as `FontSize::Px(24.0)`. For larger visual text, keep a valid raster size and scale the entity when layout and visual quality allow it.
