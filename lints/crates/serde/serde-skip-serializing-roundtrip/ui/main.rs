#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[cfg(any())]
#[derive(Serialize, Deserialize)]
struct CfgDisabledResource {
    #[serde(skip_serializing)]
    hash: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Resource {
    name: String,
    #[serde(skip_serializing)]
    hash: String,
    #[serde(skip_serializing)]
    inline_hash: String,
}

#[derive(Serialize, Deserialize)]
struct DefaultedResource {
    name: String,
    #[serde(skip_serializing, default)]
    hash: String,
}

#[derive(Serialize, Deserialize)]
struct SkippedResource {
    name: String,
    #[serde(skip)]
    hash: String,
}

#[derive(Serialize, Deserialize)]
struct SkipDeserializingResource {
    name: String,
    #[serde(skip_deserializing, skip_serializing)]
    hash: String,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct ContainerDefaultResource {
    name: String,
    #[serde(skip_serializing)]
    hash: String,
}

fn main() {}
