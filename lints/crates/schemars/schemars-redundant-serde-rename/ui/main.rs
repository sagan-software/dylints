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

#[derive(JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
#[schemars(rename_all = "camelCase")]
struct RenameAllOnly {
    request_id: String,
}

fn main() {}
