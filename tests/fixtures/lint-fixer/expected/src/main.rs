//! This fixture exercises automatic lint fixes.
//! It keeps nested module paths and imports.
//! The source provides deterministic syntax rewrites.
//! The verification pass checks the resulting source.
//! It keeps this workflow free from manual edits.
//! The fixture covers documentation and imports.
//! Its expected output is checked byte for byte.

mod nested_parent {
    pub mod child {
        pub struct NestedThing;
    }
}

use self::nested_parent::child::NestedThing;

fn main() {
    let _ = NestedThing;
}
