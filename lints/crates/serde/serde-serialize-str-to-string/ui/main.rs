// run-rustfix
// rustfix-only-machine-applicable
use serde::Serializer;

fn allocated<S>(serializer: S, value: u64) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn already_rendered<S>(serializer: S, rendered: &str) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(rendered)
}

fn collected<S>(serializer: S, value: u64) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(&value)
}

struct Inherent;

impl Inherent {
    fn to_string(&self) -> String {
        String::from("inherent")
    }
}

fn inherent<S>(serializer: S, value: &Inherent) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

struct ManualToString;

impl ToString for ManualToString {
    fn to_string(&self) -> String {
        String::from("manual")
    }
}

fn manual_to_string<S>(serializer: S, value: &ManualToString) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

struct DerefToDisplay(u64);

impl std::ops::Deref for DerefToDisplay {
    type Target = u64;

    fn deref(&self) -> &u64 {
        &self.0
    }
}

fn deref_to_display<S>(serializer: S, value: &DerefToDisplay) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn main() {}
