use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum InternallyTagged {
    Created,
    Deleted,
    Unknown,
}

#[derive(Deserialize)]
#[serde(tag = "kind", content = "data")]
enum AdjacentlyTagged {
    Created(String),
    Deleted(String),
    Other,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum ExistingOther {
    Created,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
enum ExternallyTagged {
    Created,
    Unknown,
}

#[derive(Deserialize)]
#[serde(tag = "kind", content = "data")]
enum PayloadVariant {
    Created,
    Unknown(String),
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum NotLast {
    Unknown,
    Created,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum SkippedFallback {
    Created,
    #[serde(skip_deserializing)]
    Unknown,
}

fn main() {}
