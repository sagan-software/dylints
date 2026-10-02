#![allow(dead_code)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize, Serialize as SerdeSerialize};

#[derive(Deserialize, Serialize)]
pub struct PaginatedResponse<T> {
    items: Vec<T>,
    next_cursor: Option<String>,
}

#[derive(serde::Serialize)]
pub struct QualifiedSerdeMissingSchema {
    id: String,
}

#[derive(SerdeSerialize)]
pub(crate) struct CrateVisibleDto {
    id: String,
}

#[derive(JsonSchema, Deserialize, Serialize)]
pub struct CompleteGenericResponse<T> {
    item: T,
}

pub mod nested {
    use serde::Serialize;

    #[derive(Serialize)]
    pub enum NestedMissingSchema {
        Ready { id: String },
        Pending,
    }
}

pub mod renamed_nested {
    use schemars::JsonSchema as SchemaDerive;
    use serde::Deserialize as De;

    #[derive(SchemaDerive, De)]
    pub struct CompleteNestedAlias {
        id: String,
    }
}

mod private_module {
    use serde::Serialize;

    // Other crates cannot reach this type, so it is not part of the schema surface.
    #[derive(Serialize)]
    pub struct HiddenDto {
        id: String,
    }
}

fn main() {}
