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

fn main() {}
