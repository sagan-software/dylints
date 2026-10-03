// run-rustfix
// rustfix-only-machine-applicable
#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_time::{Real, Time};

macro_rules! widen {
    ($value:expr) => {
        $value as f64
    };
}

fn elapsed(time: &Time, real: &Time<Real>) {
    let _ = time.elapsed_secs() as f64;
    let _ = 2.0 * (time.elapsed_secs()) as f64;
    let _ = f64::from(real.elapsed_secs());
    let _: f64 = time.elapsed_secs().into();
    let _ = widen!(time.elapsed_secs());
    let _ = time.elapsed_secs_f64();
    let _ = time.elapsed_secs() as f32;
    let _ = f64::from(time.delta_secs());
    let _ = f64::from(1.0_f32);
    let from: fn(f32) -> f64 = f64::from;
    let _ = from(real.elapsed_secs());
}

fn main() {}
