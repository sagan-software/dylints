use schemars::JsonSchema;
use serde::Serialize;

#[derive(JsonSchema, Serialize)]
#[serde(tag = "kind")]
#[schemars(tag = "kind")]
enum Example {
    Unit,
}

fn main() {}
