// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(expecting = "A user id.")]
struct BadUserId(String);

#[derive(Deserialize)]
#[serde(expecting = "A \"quoted\" user id.")]
struct EscapedUserId(String);

#[derive(Deserialize)]
#[serde(expecting = r#"A "raw" user id."#)]
struct RawUserId(String);

#[derive(Deserialize)]
#[serde(expecting = "UUID string.")]
struct AcronymId(String);

#[derive(Deserialize)]
#[serde(expecting = "\x41 user id")]
struct EscapedCapital(String);

#[derive(Deserialize)]
#[serde(deny_unknown_fields, expecting = "Settings.")]
enum Settings {
    Default,
}

#[derive(Deserialize)]
#[serde(expecting = "a user id")]
struct GoodUserId(String);

#[derive(Deserialize)]
#[serde(expecting = "UUID string")]
struct GoodAcronymId(String);

#[cfg(any())]
#[derive(Deserialize)]
#[serde(expecting = "Disabled.")]
struct Disabled(String);

fn main() {}
