#![allow(dead_code)]

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(expecting = "A user id.")]
struct BadUserId(String);

#[derive(Deserialize)]
#[serde(expecting = "a user id")]
struct GoodUserId(String);

fn main() {}
