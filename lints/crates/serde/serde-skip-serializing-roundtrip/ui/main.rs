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

type MaybeHash = Option<String>;

mod hash_format {
    pub fn serialize<S: serde::Serializer>(
        value: &Option<String>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(value, serializer)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<String>, D::Error> {
        serde::Deserialize::deserialize(deserializer)
    }
}

#[derive(Serialize, Deserialize)]
struct OptionalResource {
    name: String,
    #[serde(skip_serializing)]
    hash: Option<String>,
    #[serde(skip_serializing)]
    aliased_hash: MaybeHash,
    #[serde(skip_serializing, with = "hash_format")]
    custom_hash: Option<String>,
}

#[derive(Serialize, Deserialize)]
enum Event {
    Created {
        #[serde(skip_serializing)]
        hash: String,
    },
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
