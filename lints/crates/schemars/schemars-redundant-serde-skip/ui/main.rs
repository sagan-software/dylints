// run-rustfix
// rustfix-only-machine-applicable
use schemars::JsonSchema;
use serde::Serialize;

#[derive(JsonSchema, Serialize)]
struct Example {
    #[serde(skip)]
    #[schemars(skip)]
    bad: String,
    #[serde(skip)]
    good: String,
}

fn main() {}
