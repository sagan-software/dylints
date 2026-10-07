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

// A named tuple constructor with a non-default argument cannot be derived.
struct NamedTuple(u8);

impl Default for NamedTuple {
    fn default() -> Self {
        NamedTuple(7)
    }
}

// A closure returning the struct is not its resolved tuple constructor.
struct ClosureTuple(u8);

impl Default for ClosureTuple {
    fn default() -> Self {
        (|| Self(7))()
    }
}

// Whole macro-produced values remain outside the supported fix scope.
macro_rules! custom_struct {
    () => {
        GeneratedBody { value: 7 }
    };
}

struct GeneratedBody {
    value: u8,
}

impl Default for GeneratedBody {
    fn default() -> Self {
        custom_struct!()
    }
}

// Literal types beyond booleans and integers do not receive this fix.
struct CharacterField {
    value: char,
}

impl Default for CharacterField {
    fn default() -> Self {
        Self { value: 'x' }
    }
}

// Another trait's default method can have a different value contract.
trait OtherDefault {
    fn default() -> Self;
}

impl OtherDefault for u8 {
    fn default() -> Self {
        7
    }
}

struct OtherTraitField {
    value: u8,
}

impl Default for OtherTraitField {
    fn default() -> Self {
        Self {
            value: <u8 as OtherDefault>::default(),
        }
    }
}

fn main() {}
