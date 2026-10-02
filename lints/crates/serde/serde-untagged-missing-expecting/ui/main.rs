#![allow(dead_code)]

use serde::Deserialize;

#[cfg(any())]
#[derive(Deserialize)]
#[serde(untagged)]
enum CfgDisabledValue {
    Name(String),
    Id(u64),
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum UntaggedValue {
    Name(String),
    Id(u64),
}

#[derive(Deserialize)]
#[serde(untagged, expecting = "a string name or numeric id")]
enum DescribedValue {
    Name(String),
    Id(u64),
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum SerializeOnly {
    Name(String),
    Id(u64),
}

fn main() {}
