#![allow(dead_code)]

// Custom constructors can return values that differ from a derived default.
struct CustomTuple(u8);

impl Default for CustomTuple {
    fn default() -> Self {
        build_tuple()
    }
}

fn build_tuple() -> CustomTuple {
    CustomTuple(7)
}

// A closure call is not the standard Default trait method.
struct ClosureField {
    value: u8,
}

impl Default for ClosureField {
    fn default() -> Self {
        Self { value: (|| 7)() }
    }
}

// A return statement leaves no tail expression for a derive replacement.
struct ExplicitReturn(u8);

impl Default for ExplicitReturn {
    fn default() -> Self {
        return Self(7);
    }
}

// An inherent method named default does not implement the standard trait.
struct FieldValue(u8);

impl FieldValue {
    fn default() -> Self {
        Self(7)
    }
}

struct InherentDefault {
    value: FieldValue,
}

impl Default for InherentDefault {
    fn default() -> Self {
        Self {
            value: FieldValue::default(),
        }
    }
}

// Macro-generated field expressions remain outside the supported fix scope.
macro_rules! custom_value {
    () => {
        7
    };
}

struct GeneratedField {
    value: u8,
}

impl Default for GeneratedField {
    fn default() -> Self {
        Self {
            value: custom_value!(),
        }
    }
}

// A local trait implementation for a primitive cannot receive a struct derive.
trait LocalTrait {
    fn value() -> Self;
}

impl LocalTrait for u8 {
    fn value() -> Self {
        7
    }
}

// A function pointer call does not resolve to the standard Default method.
struct PointerField {
    value: u8,
}

impl Default for PointerField {
    fn default() -> Self {
        Self {
            value: (build_number as fn() -> u8)(),
        }
    }
}

fn build_number() -> u8 {
    7
}

fn main() {}
