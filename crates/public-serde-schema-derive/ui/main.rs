#![allow(dead_code)]

use schemars::JsonSchema as Schema;
use serde::{Deserialize, Deserialize as SerdeDeserialize, Serialize};

#[derive(Serialize)]
pub struct MissingSchemaStruct {
    id: String,
}

#[derive(serde::Deserialize)]
pub enum MissingSchemaEnum {
    Ready,
}

#[derive(SerdeDeserialize)]
pub struct MissingRenamedSerdeDerive {
    id: String,
}

#[derive(Schema, Serialize)]
pub struct HasSchemaStruct {
    id: String,
}

#[derive(schemars::JsonSchema, Deserialize)]
pub enum HasQualifiedSchema {
    Ready,
}

#[derive(Schema, SerdeDeserialize)]
pub enum HasRenamedSchema {
    Ready,
}

#[derive(Serialize)]
struct PrivateDto {
    id: String,
}

#[derive(Clone)]
pub struct NotSerdeDto {
    id: String,
}

#[cfg(any())]
#[derive(Serialize)]
pub struct CfgDisabledMissingSchema {
    id: String,
}

pub mod lookalikes {
    #[derive(Clone)]
    pub struct Serialize;

    #[derive(Clone)]
    pub struct JsonSchema;

    #[derive(Clone)]
    pub struct LocalLookalikeNamesAreNotDerives {
        serialize: Serialize,
        json_schema: JsonSchema,
    }
}

fn main() {}
