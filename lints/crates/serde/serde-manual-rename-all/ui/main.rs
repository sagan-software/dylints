use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct BothDirections {
    #[serde(rename = "firstName")]
    first_name: String,
    #[serde(rename = "lastName")]
    last_name: String,
}

#[derive(Serialize)]
struct SerializeOnly {
    #[serde(rename(serialize = "user-id"))]
    user_id: String,
    #[serde(rename(serialize = "team-id"))]
    team_id: String,
}

#[derive(Deserialize)]
struct DifferentDirections {
    #[serde(rename(deserialize = "FIRST-NAME"))]
    first_name: String,
    #[serde(rename(deserialize = "LAST-NAME"))]
    last_name: String,
}

#[derive(Serialize)]
struct PartialRename {
    #[serde(rename = "firstName")]
    first_name: String,
    last_name: String,
}

#[derive(Serialize)]
struct MixedConventions {
    #[serde(rename = "firstName")]
    first_name: String,
    #[serde(rename = "LAST_NAME")]
    last_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExistingContainerRule {
    #[serde(rename = "firstName")]
    first_name: String,
    #[serde(rename = "lastName")]
    last_name: String,
}

#[derive(Serialize)]
struct OneField {
    #[serde(rename = "userId")]
    user_id: String,
}

fn main() {}
