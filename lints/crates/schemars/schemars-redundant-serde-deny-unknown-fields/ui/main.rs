// run-rustfix
// rustfix-only-machine-applicable
use schemars::JsonSchema;
use serde::Serialize;

#[derive(JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
struct Example {
    field: String,
}

fn main() {}
