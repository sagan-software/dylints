use std::fmt;

use serde::de::{Deserializer, Visitor};

struct AnyVisitor;

impl<'de> Visitor<'de> for AnyVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any value")
    }
}

fn dynamic<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(AnyVisitor)
}

struct StringVisitor;

impl<'de> Visitor<'de> for StringVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string")
    }
}

fn typed<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(StringVisitor)
}

trait LocalDeserializer {
    fn deserialize_any(self) -> Result<(), ()>;
}

fn local<D: LocalDeserializer>(deserializer: D) -> Result<(), ()> {
    deserializer.deserialize_any()
}

fn main() {}
