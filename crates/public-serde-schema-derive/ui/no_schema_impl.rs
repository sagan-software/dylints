#![allow(dead_code)]

use serde::Serialize;

#[derive(Serialize)]
pub struct NoSchemaImplementation {
    value: String,
}

fn main() {}
