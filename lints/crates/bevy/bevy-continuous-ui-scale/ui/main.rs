#![allow(dead_code, unused_variables, unused_mut)]

use bevy::app::{App, Startup, Update};
use bevy::ecs::prelude::*;
use bevy::ecs::schedule::common_conditions::{resource_exists, run_once};
use bevy::math::Vec3;
use bevy::time::{Fixed, Time};
use bevy::transform::components::Transform;
use bevy::ui::UiScale as BevyUiScale;

mod custom_schedule {
    use bevy::ecs::schedule::ScheduleLabel;

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Update;
}

#[derive(Resource)]
struct SelectedScale(f32);

#[derive(Resource)]
struct UiChoice(bool);

#[derive(Resource)]
struct UiScale(f32);

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

fn animate(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    let delta = time.delta_secs();
    scale.0 += delta * 0.1;
}

fn animate_once(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn animate_group_left(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn animate_group_right(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn animate_inner_left(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn animate_inner_right(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn animate_configured(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn animate_once_and_left(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn animate_once_and_right(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn animate_distributive_once(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn animate_unknown(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn animate_resource_condition(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

mod lookalike_condition {
    pub(crate) fn run_once() -> bool {
        true
    }
}

fn animate_lookalike(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn animate_or(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn animate_both(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn always_condition() -> bool {
    true
}

struct ConditionFactory;

impl ConditionFactory {
    fn always(self) -> impl Fn() -> bool {
        || true
    }
}

fn animate_method_condition(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn whole_resource(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    *scale = BevyUiScale(1.0 + time.elapsed_secs().sin());
}

fn finite_choice(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = if time.elapsed_secs() > 2.0 { 1.5 } else { 1.0 };
}

fn finite_match(mut scale: ResMut<BevyUiScale>, selected: Res<SelectedScale>) {
    scale.0 = match selected.0 > 1.0 {
        true => 1.5,
        false => 1.0,
    };
}

fn match_local_select(
    mut scale: ResMut<BevyUiScale>,
    time: Res<Time>,
    selected: Res<SelectedScale>,
) {
    let mut value = time.elapsed_secs();
    match selected.0 > 1.0 {
        true => value = 1.5,
        false => value = 1.0,
    }
    scale.0 = value;
}

fn match_local_source(
    mut scale: ResMut<BevyUiScale>,
    time: Res<Time>,
    selected: Res<SelectedScale>,
) {
    let mut value = 1.0;
    match selected.0 > 1.0 {
        true => value = time.elapsed_secs(),
        false => value = 1.5,
    }
    scale.0 = value;
}

fn selected(mut scale: ResMut<BevyUiScale>, selected: Res<SelectedScale>) {
    scale.0 = selected.0;
}

fn constant(mut scale: ResMut<BevyUiScale>) {
    scale.0 = 1.25;
}

fn arithmetic_sources(
    mut scale: ResMut<BevyUiScale>,
    time: Res<Time>,
    selected: Res<SelectedScale>,
) {
    let factor = 1.5;
    let past_end = time.elapsed_secs() > 1.0;
    let mut canceled_scale = time.elapsed_secs();
    canceled_scale -= canceled_scale;
    scale.0 = canceled_scale;
    for _ in 0..0 {
        scale.0 = time.elapsed_secs();
    }
    scale.0 = time.elapsed_secs() + ADDITIVE_OFFSET;
    scale.0 = SUBTRACTIVE_OFFSET - time.elapsed_secs();
    scale.0 = SCALE_FACTOR * time.elapsed_secs();
    scale.0 = time.elapsed_secs() / DIVISOR;
    scale.0 = -time.elapsed_secs();
    scale.0 = time.elapsed_secs_f64() as f32;
    scale.0 = if selected.0 > 1.0 {
        time.elapsed_secs()
    } else {
        1.0
    };
    scale.0 = if selected.0 > 1.0 {
        1.0
    } else {
        time.elapsed_secs()
    };
    scale.0 = match selected.0 > 1.0 {
        true => time.elapsed_secs(),
        false => 1.0,
    };
    scale.0 = match selected.0 > 1.0 {
        true => 1.0,
        false => time.elapsed_secs(),
    };
    scale.0 = time.elapsed_secs() * factor;
    scale.0 = time.elapsed_secs() * Factors::VALUE;
    scale.0 = time.elapsed_secs() * std::f32::consts::PI;
    scale.0 = time.elapsed_secs() * ZERO_FACTOR;
    scale.0 = time.elapsed_secs() * UNDERFLOW_FACTOR;
    scale.0 = time.elapsed_secs() * NONZERO_FACTOR;
    scale.0 = (time.elapsed_secs_f64() * ZERO_F64_FACTOR) as f32;
    scale.0 = (time.elapsed_secs_f64() * NONZERO_F64_FACTOR) as f32;
    scale.0 = (time.elapsed_secs_f64() * NEGATIVE_F64_FACTOR) as f32;
    scale.0 = (time.elapsed_secs_f64() * ARITHMETIC_F64_FACTOR) as f32;
    scale.0 = (time.elapsed_secs_f64() * REMAINDER_F64_FACTOR) as f32;
    scale.0 = time.elapsed_secs() - time.elapsed_secs();
    scale.0 = time.elapsed_secs() / time.elapsed_secs();
    scale.0 = time.elapsed_secs() * (2.0 % 1.5);
    scale.0 = time.elapsed_secs() % 10.0;
    scale.0 = time.elapsed_secs() * DEEP_FACTOR;
    scale.0 = time.elapsed_secs() * MAX_DEPTH_FACTOR;
    if past_end {
        scale.0 = 1.0;
    }
}

fn quantized(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs().floor();
}

fn more_time_sources(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs_f64() as f32;
    scale.0 = time.elapsed_secs_wrapped();
    scale.0 = time.elapsed_secs_wrapped_f64() as f32;
    scale.0 = time.delta_secs_f64() as f32;
    scale.0 = time.elapsed().as_secs_f32();
    scale.0 = time.delta().as_secs_f32();
}

fn standard_float_methods(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time
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
}

fn let_else_scale(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    let Some(value) = [time.elapsed_secs()].first().copied() else {
        return;
    };
    scale.0 = value;
}

fn uninitialized_local(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    let value: f32;
    value = time.elapsed_secs();
    scale.0 = value;
}

fn generic_trait_factor<T: Factor>(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs() * T::VALUE;
}

fn generic_default_trait_factor<T: DefaultFactor>(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs() * T::VALUE;
}

fn overwritten_local(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    let mut value = time.elapsed_secs();
    value = 1.0;
    scale.0 = value;
}

fn compound_add(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    let mut value = time.elapsed_secs();
    value += 1.0;
    scale.0 = value;
}

fn compound_subtract_and_divide(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    let mut value = 1.0;
    value -= time.elapsed_secs();
    value /= 2.0;
    scale.0 = value;
}

fn unsupported_compound_remainder(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    let mut value = time.elapsed_secs();
    value %= 10.0;
    scale.0 = value;
}

fn compound_dynamic_rhs(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    let mut value = 1.0;
    value *= time.elapsed_secs();
    scale.0 = value;
}

fn compound_zero(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    let mut value = time.elapsed_secs();
    value *= 0.0;
    scale.0 = value;
}

fn discrete_local_select(
    mut scale: ResMut<BevyUiScale>,
    time: Res<Time>,
    selected: Res<SelectedScale>,
) {
    let mut value = time.elapsed_secs();
    if selected.0 > 1.0 {
        value = 1.5;
    } else {
        value = 1.0;
    }
    scale.0 = value;
}

fn conditional_local_source(
    mut scale: ResMut<BevyUiScale>,
    time: Res<Time>,
    selected: Res<SelectedScale>,
) {
    let mut value = 1.0;
    if selected.0 > 1.0 {
        value = time.elapsed_secs();
    }
    scale.0 = value;
}

fn zero_factor(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs() * 0.0;
}

fn unused_closure(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    let update = || {
        scale.0 = time.elapsed_secs();
    };
    let _ = update;
}

fn distance(mut scale: ResMut<BevyUiScale>, transforms: Query<&Transform>) {
    for transform in &transforms {
        scale.0 = transform.translation.distance(Vec3::ZERO);
    }
}

fn constant_distances(mut scale: ResMut<BevyUiScale>) {
    scale.0 = Vec3::X.distance(Vec3::ZERO);
    scale.0 = SOURCE_POINT.distance(TARGET_POINT);
}

fn fixed_delta(mut scale: ResMut<BevyUiScale>, time: Res<Time<Fixed>>) {
    scale.0 = time.delta_secs();
}

fn duration_as_secs_f64(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed().as_secs_f64() as f32;
}

fn custom_schedule_animation(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn startup(mut scale: ResMut<BevyUiScale>, time: Res<Time>) {
    scale.0 = 1.0 + time.elapsed_secs();
}

fn lookalike(mut scale: ResMut<UiScale>, time: Res<Time>) {
    scale.0 = time.elapsed_secs();
}

fn configure(app: &mut App) {
    app.add_systems(
        Update,
        (
            animate,
            whole_resource,
            finite_choice,
            finite_match,
            match_local_select,
            match_local_source,
            selected,
            constant,
            overwritten_local,
            compound_add,
            compound_dynamic_rhs,
            compound_zero,
            discrete_local_select,
            conditional_local_source,
            zero_factor,
            unused_closure,
            distance,
            constant_distances,
            fixed_delta,
            lookalike,
        ),
    );
    app.add_systems(
        Update,
        (
            arithmetic_sources,
            more_time_sources,
            standard_float_methods,
            let_else_scale,
            uninitialized_local,
            quantized,
            compound_subtract_and_divide,
            unsupported_compound_remainder,
            duration_as_secs_f64,
        ),
    );
    app.add_systems(custom_schedule::Update, custom_schedule_animation);
    app.add_systems(Startup, startup);
    app.add_systems(custom_schedule::Update, startup);
    app.add_systems(Update, animate_once.run_if(run_once));
    app.add_systems(
        Update,
        (animate_group_left, animate_group_right)
            .before(animate)
            .run_if(run_once),
    );
    app.add_systems(
        Update,
        (
            animate_inner_left.run_if(run_once),
            animate_inner_right.run_if(run_once),
        ),
    );
    app.add_systems(Update, animate_configured.after(animate).run_if(run_once));
    app.add_systems(Update, animate_configured.run_if(|| true));
    app.add_systems(
        Update,
        animate_once_and_left.run_if(run_once.and_then(always_condition)),
    );
    app.add_systems(
        Update,
        animate_once_and_right.run_if(always_condition.and_then(run_once)),
    );
    app.add_systems(
        Update,
        animate_distributive_once.distributive_run_if(run_once),
    );
    app.add_systems(Update, animate_unknown.run_if(always_condition));
    app.add_systems(
        Update,
        animate_resource_condition.run_if(resource_exists::<Time>),
    );
    app.add_systems(
        Update,
        animate_method_condition.run_if(ConditionFactory.always()),
    );
    app.add_systems(
        Update,
        animate_lookalike.run_if(lookalike_condition::run_once),
    );
    app.add_systems(
        Update,
        animate_or.run_if(run_once.or_else(always_condition)),
    );
    app.add_systems(Update, animate_both.run_if(run_once));
    app.add_systems(Update, animate_both);
}

fn main() {
    let _ = UiChoice(false);
    let _ = ChosenFactor;
    let mut app = App::new();
    configure(&mut app);
}
