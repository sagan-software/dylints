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

fn path_call<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    Deserializer::deserialize_any(deserializer, AnyVisitor)
}

fn closure_call<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    let call = |value: D, visitor: AnyVisitor| Ok::<_, D::Error>((value, visitor));
    let _ = call(deserializer, AnyVisitor);
    let _ = (|| Ok::<_, D::Error>(()))();
    Ok(())
}

fn local_call<'de, D>(_: D, _: AnyVisitor) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    Ok(())
}

fn function_pointer_call<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    let _ = local_call(deserializer, AnyVisitor);
    Ok(())
}

struct Wrapper<D>(D);

impl<'de, D: Deserializer<'de>> Deserializer<'de> for Wrapper<D> {
    type Error = D::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.0.deserialize_any(visitor)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

trait LocalDeserializer {
    fn deserialize_any(self) -> Result<(), ()>;
}

fn local<D: LocalDeserializer>(deserializer: D) -> Result<(), ()> {
    deserializer.deserialize_any()
}

fn main() {}
