use schemars::JsonSchema;
use serde::Serialize;

#[derive(JsonSchema, Serialize)]
#[serde(transparent)]
#[schemars(transparent)]
struct Example(String);

fn main() {}
