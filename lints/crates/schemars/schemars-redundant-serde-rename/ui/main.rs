use schemars::JsonSchema;
use serde::Serialize;

#[derive(JsonSchema, Serialize)]
struct Example {
    #[serde(rename = "id")]
    #[schemars(rename = "id")]
    bad: String,
    #[serde(rename = "wire")]
    #[schemars(rename = "schema")]
    good: String,
}

fn main() {}
