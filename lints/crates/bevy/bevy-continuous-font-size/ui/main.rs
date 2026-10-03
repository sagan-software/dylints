#![allow(dead_code, unused_assignments, unused_variables, unused_mut)]

use bevy_app::{App, Startup, Update};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::common_conditions::run_once;
use bevy_math::Vec3;
use bevy_text::{FontSize, TextFont};
use bevy_time::{Fixed, Time as BevyClock};
use bevy_transform::components::Transform;

type LabelFont = TextFont;

const SOURCE_POINT: Vec3 = Vec3::X;
const TARGET_POINT: Vec3 = Vec3::ZERO;
const ADDITIVE_OFFSET: f32 = 1.0 + 0.5;
const SUBTRACTIVE_OFFSET: f32 = 2.0 - 0.5;
const SCALE_FACTOR: f32 = -2.0 * 0.5;
const DIVISOR: f32 = 4.0 / 2.0;
const ZERO_FACTOR: f32 = 16_777_217.0_f32 - 16_777_216.0_f32;
const UNDERFLOW_FACTOR: f32 = 1.0e-30_f32 * 1.0e-30_f32;
const NONZERO_FACTOR: f32 = 16_777_218.0_f32 - 16_777_216.0_f32;
const ZERO_F64_FACTOR: f64 = 16_777_216.0_f64 - 16_777_216.0_f64;
const NONZERO_F64_FACTOR: f64 = 16_777_217.0_f64 - 16_777_216.0_f64;
const NEGATIVE_F64_FACTOR: f64 = -1.0_f64;
const ARITHMETIC_F64_FACTOR: f64 = (2.0_f64 + 4.0_f64) * 3.0_f64 / 18.0_f64 - 0.0_f64;
const REMAINDER_F64_FACTOR: f64 = 2.0_f64 % 1.5_f64;
const MAX_DEPTH_FACTOR: f32 =
    1.0 + 0.0 + 0.0 + 0.0 + 0.0 + 0.0 + 0.0 + 0.0 + 0.0 + 0.0 + 0.0 + 0.0 + 0.0 + 0.0 + 0.0 + 0.0;
const DEEP_FACTOR: f32 = 1.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0
    + 0.0;

trait Factor {
    const VALUE: f32;
}

trait DefaultFactor {
    const VALUE: f32 = 2.0;
}

struct ChosenFactor;

struct Factors;

impl Factors {
    const VALUE: f32 = 1.5;
}

impl Factor for ChosenFactor {
    const VALUE: f32 = 1.5;
}

impl DefaultFactor for ChosenFactor {
    const VALUE: f32 = 1.5;
}

mod custom_schedule {
    use bevy_ecs::schedule::ScheduleLabel;

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Update;
}

#[derive(Resource)]
struct UiChoice(bool);

#[derive(Component)]
struct Lookalike {
    font_size: f32,
}

#[derive(Resource)]
struct LookClock;

impl LookClock {
    fn elapsed_secs(&self) -> f32 {
        1.0
    }
}

fn animate_time(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    let size = clock.elapsed_secs() * 24.0;
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size);
    }
}

fn animate_once(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(clock.elapsed_secs());
    }
}

fn animate_distance(mut labels: Query<(&Transform, &mut TextFont)>) {
    for (transform, mut label) in &mut labels {
        label.font_size = FontSize::Px(transform.translation.distance(Vec3::ZERO));
    }
}

fn constant_distances(mut labels: Query<&mut TextFont>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(Vec3::X.distance(Vec3::ZERO));
        label.font_size = FontSize::Px(SOURCE_POINT.distance(TARGET_POINT));
    }
}

fn finite_choice(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        label.font_size = if clock.elapsed_secs() > 1.0 {
            FontSize::Px(18.0)
        } else {
            FontSize::Px(24.0)
        };
    }
}

fn finite_match(mut labels: Query<&mut TextFont>, choice: Res<UiChoice>) {
    let size = match choice.0 {
        true => FontSize::Px(18.0),
        false => FontSize::Px(24.0),
    };
    for mut label in &mut labels {
        label.font_size = size;
    }
}

fn match_local_select(
    mut labels: Query<&mut TextFont>,
    clock: Res<BevyClock>,
    choice: Res<UiChoice>,
) {
    let mut size = clock.elapsed_secs();
    match choice.0 {
        true => size = 18.0,
        false => size = 24.0,
    }
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size);
    }
}

fn match_local_source(
    mut labels: Query<&mut TextFont>,
    clock: Res<BevyClock>,
    choice: Res<UiChoice>,
) {
    let mut size = 24.0;
    match choice.0 {
        true => size = clock.elapsed_secs(),
        false => size = 18.0,
    }
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size);
    }
}

fn quantized(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(clock.elapsed_secs().floor());
    }
}

fn constant_size(mut labels: Query<&mut TextFont>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(24.0);
    }
}

fn arithmetic_sources(
    mut labels: Query<&mut TextFont>,
    clock: Res<BevyClock>,
    choice: Res<UiChoice>,
) {
    let factor = 1.5;
    let past_end = clock.elapsed_secs() > 1.0;
    let mut canceled_size = clock.elapsed_secs();
    canceled_size -= canceled_size;
    for mut label in &mut labels {
        label.font_size = FontSize::Px(canceled_size);
        for _ in 0..0 {
            label.font_size = FontSize::Px(clock.elapsed_secs());
        }
        label.font_size = FontSize::Px(clock.elapsed_secs() + ADDITIVE_OFFSET);
        label.font_size = FontSize::Px(SUBTRACTIVE_OFFSET - clock.elapsed_secs());
        label.font_size = FontSize::Px(SCALE_FACTOR * clock.elapsed_secs());
        label.font_size = FontSize::Px(clock.elapsed_secs() / DIVISOR);
        label.font_size = FontSize::Px(-clock.elapsed_secs());
        label.font_size = FontSize::Px(clock.elapsed_secs_f64() as f32);
        label.font_size = FontSize::Px(if choice.0 { clock.elapsed_secs() } else { 24.0 });
        label.font_size = FontSize::Px(if choice.0 { 24.0 } else { clock.elapsed_secs() });
        label.font_size = FontSize::Px(match choice.0 {
            true => clock.elapsed_secs(),
            false => 24.0,
        });
        label.font_size = FontSize::Px(match choice.0 {
            true => 24.0,
            false => clock.elapsed_secs(),
        });
        label.font_size = FontSize::Px(clock.elapsed_secs() * factor);
        label.font_size = FontSize::Px(clock.elapsed_secs() * Factors::VALUE);
        label.font_size = FontSize::Px(clock.elapsed_secs() * std::f32::consts::PI);
        label.font_size = FontSize::Px(clock.elapsed_secs() * ZERO_FACTOR);
        label.font_size = FontSize::Px(clock.elapsed_secs() * UNDERFLOW_FACTOR);
        label.font_size = FontSize::Px(clock.elapsed_secs() * NONZERO_FACTOR);
        label.font_size = FontSize::Px((clock.elapsed_secs_f64() * ZERO_F64_FACTOR) as f32);
        label.font_size = FontSize::Px((clock.elapsed_secs_f64() * NONZERO_F64_FACTOR) as f32);
        label.font_size = FontSize::Px((clock.elapsed_secs_f64() * NEGATIVE_F64_FACTOR) as f32);
        label.font_size = FontSize::Px((clock.elapsed_secs_f64() * ARITHMETIC_F64_FACTOR) as f32);
        label.font_size = FontSize::Px((clock.elapsed_secs_f64() * REMAINDER_F64_FACTOR) as f32);
        label.font_size = FontSize::Px(clock.elapsed_secs() - clock.elapsed_secs());
        label.font_size = FontSize::Px(clock.elapsed_secs() / clock.elapsed_secs());
        label.font_size = FontSize::Px(clock.elapsed_secs() * (2.0 % 1.5));
        label.font_size = FontSize::Px(clock.elapsed_secs() % 10.0);
        label.font_size = FontSize::Px(clock.elapsed_secs() * DEEP_FACTOR);
        label.font_size = FontSize::Px(clock.elapsed_secs() * MAX_DEPTH_FACTOR);
        if past_end {
            label.font_size = FontSize::Px(24.0);
        }
    }
}

fn dynamic_viewport_size(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Vh(clock.elapsed_secs());
    }
}

fn quantized_size(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(clock.elapsed_secs().floor());
    }
}

fn more_time_sources(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(clock.elapsed_secs_f64() as f32);
        label.font_size = FontSize::Px(clock.elapsed_secs_wrapped());
        label.font_size = FontSize::Px(clock.elapsed_secs_wrapped_f64() as f32);
        label.font_size = FontSize::Px(clock.delta_secs_f64() as f32);
        label.font_size = FontSize::Px(clock.elapsed().as_secs_f32());
        label.font_size = FontSize::Px(clock.delta().as_secs_f32());
    }
}

fn standard_float_methods(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    let value = clock
        .elapsed_secs()
        .abs()
        .acos()
        .asin()
        .atan()
        .atan2(1.0)
        .cbrt()
        .cos()
        .cosh()
        .exp()
        .exp2()
        .hypot(1.0)
        .ln()
        .log(2.0)
        .log10()
        .log2()
        .powf(2.0)
        .powi(2)
        .recip()
        .sin()
        .sinh()
        .sqrt()
        .tan()
        .tanh()
        .to_degrees()
        .to_radians();
    for mut label in &mut labels {
        label.font_size = FontSize::Px(value);
        label.font_size = FontSize::Px(clock.elapsed_secs().signum());
    }
}

fn default_text_font(mut labels: Query<&mut TextFont>) {
    for mut label in &mut labels {
        *label = TextFont::default();
    }
}

fn let_else_label(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    let Some(mut label) = labels.iter_mut().next() else {
        return;
    };
    label.font_size = FontSize::Px(clock.elapsed_secs());
}

fn uninitialized_local(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    let size: f32;
    size = clock.elapsed_secs();
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size);
    }
}

fn generic_trait_factor<T: Factor>(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(clock.elapsed_secs() * T::VALUE);
    }
}

fn generic_default_trait_factor<T: DefaultFactor>(
    mut labels: Query<&mut TextFont>,
    clock: Res<BevyClock>,
) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(clock.elapsed_secs() * T::VALUE);
    }
}

fn replace_with_builder(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        *label = TextFont::default().with_font_size(clock.elapsed_secs() * 1.5);
    }
}

fn replace_with_struct(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        *label = TextFont {
            font_size: FontSize::Px(clock.delta_secs()),
            ..Default::default()
        };
    }
}

fn integer_quantized(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    let size = clock.elapsed_secs() as i32;
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size as f32);
    }
}

fn lookalike_time(mut labels: Query<&mut TextFont>, clock: Res<LookClock>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(clock.elapsed_secs());
    }
}

fn aliased_component(mut labels: Query<&mut LabelFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(clock.delta_secs());
    }
}

fn overwritten_local(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    let mut size = clock.elapsed_secs();
    size = 24.0;
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size);
    }
}

fn compound_add(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    let mut size = clock.elapsed_secs();
    size += 1.0;
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size);
    }
}

fn compound_subtract_and_divide(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    let mut size = 24.0;
    size -= clock.elapsed_secs();
    size /= 2.0;
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size);
    }
}

fn unsupported_compound_remainder(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    let mut size = clock.elapsed_secs();
    size %= 10.0;
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size);
    }
}

fn compound_dynamic_rhs(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    let mut size = 24.0;
    size *= clock.elapsed_secs();
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size);
    }
}

fn compound_zero(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    let mut size = clock.elapsed_secs();
    size *= 0.0;
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size);
    }
}

fn discrete_local_select(
    mut labels: Query<&mut TextFont>,
    clock: Res<BevyClock>,
    choice: Res<UiChoice>,
) {
    let mut size = clock.elapsed_secs();
    if choice.0 {
        size = 18.0;
    } else {
        size = 24.0;
    }
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size);
    }
}

fn conditional_local_source(
    mut labels: Query<&mut TextFont>,
    clock: Res<BevyClock>,
    choice: Res<UiChoice>,
) {
    let mut size = 24.0;
    if choice.0 {
        size = clock.elapsed_secs();
    }
    for mut label in &mut labels {
        label.font_size = FontSize::Px(size);
    }
}

fn zero_factor(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(clock.elapsed_secs() * 0.0);
    }
}

fn fixed_delta(mut labels: Query<&mut TextFont>, clock: Res<BevyClock<Fixed>>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(clock.delta_secs());
    }
}

fn unused_closure(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    let update = || {
        for mut label in &mut labels {
            label.font_size = FontSize::Px(clock.elapsed_secs());
        }
    };
    let _ = update;
}

fn unregistered(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(clock.elapsed_secs());
    }
}

fn startup_size(mut labels: Query<&mut TextFont>, clock: Res<BevyClock>) {
    for mut label in &mut labels {
        label.font_size = FontSize::Px(clock.elapsed_secs());
    }
}

fn transform_scale(mut transforms: Query<&mut Transform>, clock: Res<BevyClock>) {
    for mut transform in &mut transforms {
        transform.scale = Vec3::splat(clock.elapsed_secs());
    }
}

fn lookalike_field(mut values: Query<&mut Lookalike>, clock: Res<BevyClock>) {
    for mut value in &mut values {
        value.font_size = clock.elapsed_secs();
    }
}

fn configure(app: &mut App) {
    app.add_systems(
        Update,
        (
            animate_time,
            animate_distance,
            constant_distances,
            finite_choice,
            finite_match,
            match_local_select,
            match_local_source,
            quantized,
            constant_size,
            replace_with_builder,
            replace_with_struct,
            arithmetic_sources,
            more_time_sources,
            standard_float_methods,
            default_text_font,
            let_else_label,
            uninitialized_local,
            dynamic_viewport_size,
            quantized_size,
        ),
    );
    app.add_systems(
        Update,
        (
            integer_quantized,
            transform_scale,
            lookalike_field,
            lookalike_time,
            aliased_component,
            overwritten_local,
            discrete_local_select,
            conditional_local_source,
            zero_factor,
            compound_add,
            compound_subtract_and_divide,
            unsupported_compound_remainder,
            compound_dynamic_rhs,
            compound_zero,
            fixed_delta,
            unused_closure,
        ),
    );
    app.add_systems(Startup, startup_size);
    app.add_systems(Update, animate_once.run_if(run_once));
    app.add_systems(custom_schedule::Update, unregistered);
}

fn main() {
    let _ = UiChoice(false);
    let _ = ChosenFactor;
    let mut app = App::new();
    configure(&mut app);
}
