// run-rustfix
// rustfix-only-machine-applicable
use schemars::JsonSchema;
use serde::Serialize;

#[derive(JsonSchema, Serialize)]
#[serde(tag = "kind")]
#[schemars(tag = "kind")]
enum Example {
    Unit,
}

#[derive(JsonSchema, Serialize)]
#[serde(tag = "type")]
#[schemars(tag = "event")]
enum TagOverride {
    Unit,
}

fn local_tag() {
    #[derive(JsonSchema, Serialize)]
    #[serde(tag = r#"https://kind,name"#)]
    #[schemars(content = "schema-payload", /* keep this content key */ tag = "https://kind,name",)]
    enum Local {
        Created(String),
    }

    let _local = Local::Created(String::new());
}

fn main() {
    local_tag();
}
