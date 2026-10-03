#![allow(dead_code, unused_variables, unused_mut)]

use bevy_text::FontSize;
use bevy_text::FontSize as PixelScale;

const TOO_SMALL: f32 = -1.0;
const TOO_LARGE: f32 = 1000.0 + 1.0;
const HALF_INTEGER: i32 = 2001 / 2;
const BELOW_BY_DIVISION: f32 = 1.0 / -2.0;
const ABOVE_BY_MULTIPLICATION: f32 = 501.0 * 2.0;
const ABOVE_BY_SUBTRACTION: f32 = 1002.0 - 1.0;
const NEAR_DEPTH: f32 = 1001.0
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
const TOO_DEEP: f32 = 1001.0
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

struct Limits;

impl Limits {
    const ZERO: f32 = 0.0;
}

trait FontSizeValue {
    const PX: f32;
}

trait DefaultFontSizeValue {
    const PX: f32 = 1001.0;
}

struct SmallFontSize;

impl DefaultFontSizeValue for SmallFontSize {
    const PX: f32 = 20.0;
}

mod unrelated {
    pub enum FontSize {
        Px(f32),
    }
}

fn main() {
    let _negative = PixelScale::Px(TOO_SMALL);
    let _zero = FontSize::Px(0.0);
    let _above = FontSize::Px(TOO_LARGE);
    let _cast = FontSize::Px(1001_i32 as f32);
    let _division = FontSize::Px(BELOW_BY_DIVISION);
    let _multiplication = FontSize::Px(ABOVE_BY_MULTIPLICATION);
    let _subtraction = FontSize::Px(ABOVE_BY_SUBTRACTION);
    let _near_depth = FontSize::Px(NEAR_DEPTH);
    let _associated_constant = FontSize::Px(Limits::ZERO);
    let _braced = FontSize::Px({ 1001.0 });
    let _block_with_statement = FontSize::Px({
        let size = 1001.0;
        size
    });
    let _deep = FontSize::Px(TOO_DEEP);
    let _boundary = FontSize::Px(1000.0);
    let _ordinary = FontSize::Px(32.0);
    let _integer_division = FontSize::Px((2001 / 2) as f32);
    let _integer_constant_division = FontSize::Px(HALF_INTEGER as f32);
    let _f64_arithmetic_narrowed = FontSize::Px((16_777_216.5_f64 - 16_777_216.0_f64) as f32);
    let _unsupported_operator = FontSize::Px(1001.0 % 1000.0);
    let _conditional = FontSize::Px(if true { 1001.0 } else { 1002.0 });
    let _matched = FontSize::Px(match true {
        true => 1001.0,
        false => 1002.0,
    });
    let _runtime = FontSize::Px(runtime_value());
    let runtime_size = runtime_value();
    let _runtime_local = FontSize::Px(runtime_size);
    let _closure_call = FontSize::Px((|| 1001.0)());
    let _external_constant = FontSize::Px(f32::INFINITY);
    let _large_viewport = FontSize::Vh(2000.0);
    let _unrelated = unrelated::FontSize::Px(-1.0);
}

fn runtime_value() -> f32 {
    1001.0
}

fn from_generic_associated_constant<T: FontSizeValue>() -> FontSize {
    FontSize::Px(T::PX)
}

fn from_overridden_default<T: DefaultFontSizeValue>() -> FontSize {
    FontSize::Px(T::PX)
}
