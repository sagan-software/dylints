use schemars::JsonSchema;
use serde::Serialize;

#[derive(JsonSchema, Serialize)]
struct Example {
    #[serde(default)]
    #[schemars(default)]
    bad: String,
    #[serde(default)]
    good: String,
}

fn main() {}
