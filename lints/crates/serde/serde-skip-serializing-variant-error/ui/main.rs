#![allow(dead_code)]

use serde::Serialize;

#[cfg(any())]
#[derive(Serialize)]
enum CfgDisabledEvent {
    #[serde(skip_serializing)]
    Internal,
}

#[derive(serde::Serialize)]
enum Event {
    Sent,
    #[serde(skip_serializing)]
    Internal,
    #[serde(skip)]
    FullySkipped,
}

#[derive(Serialize)]
enum FullySerializable {
    Sent,
    Internal,
}

#[derive(serde::Deserialize)]
enum DeserializeOnly {
    #[serde(skip_serializing)]
    Internal,
    #[serde(skip)]
    Hidden,
}

#[derive(Serialize)]
enum DeserializeSkipOnly {
    Sent,
    #[serde(skip_deserializing)]
    Outbound,
}

fn main() {}
