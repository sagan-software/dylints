// run-rustfix
// rustfix-only-machine-applicable
use schemars::JsonSchema;
use serde::Serialize;

#[derive(JsonSchema, Serialize)]
#[serde(transparent)]
#[schemars(transparent)]
struct Example(String);

fn local_transparent() {
    #[derive(JsonSchema, Serialize)]
    #[serde(transparent, rename = "LocalId")]
    #[schemars(rename = "SchemaId", /* keep this name */ transparent)]
    struct Local(String);

    let _local = Local(String::new());
}

fn main() {
    local_transparent();
}
