#![doc = "This fixture exercises automatic lint fixes."]
#![doc = "It keeps nested module paths and imports."]
#![doc = "The source provides deterministic syntax rewrites."]
#![doc = "The verification pass checks the resulting source."]
#![doc = "It keeps this workflow free from manual edits."]
#![doc = "The fixture covers documentation and imports."]
#![doc = "Its expected output is checked byte for byte."]

mod nested_parent {
    pub mod child {
        pub struct NestedThing;
    }
}

use nested_parent::child::NestedThing;

fn main() {
    let _ = NestedThing;
}
