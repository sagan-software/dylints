// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[cfg(any())]
#[derive(Serialize)]
struct CfgDisabledSerializeOnly {
    #[serde(default)]
    value: String,
}

#[derive(serde::Serialize)]
#[serde(default)]
struct SerializeOnlyContainer {
    value: String,
}

#[derive(Serialize)]
struct SerializeOnly {
    #[serde(alias = "old_value")]
    value: String,
}

#[derive(Deserialize)]
struct DeserializeOnly {
    #[serde(skip_serializing_if = "String::is_empty")]
    value: String,
}

#[derive(Deserialize)]
enum DeserializeOnlyEnum {
    #[serde(skip_serializing)]
    Hidden,
}

#[derive(Serialize, Deserialize)]
struct BothDirections {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    value: String,
}

fn main() {}
