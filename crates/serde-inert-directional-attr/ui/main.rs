// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[cfg(any())]
#[derive(Serialize)]
struct CfgDisabledSerializeOnly {
    #[serde(default)]
    value: String,
}

#[derive(serde::Serialize)]
#[serde(default)]
struct SerializeOnlyContainer {
    value: String,
}

#[derive(Serialize)]
struct SerializeOnly {
    #[serde(alias = "old_value")]
    value: String,
}

#[derive(Deserialize)]
struct DeserializeOnly {
    #[serde(skip_serializing_if = "String::is_empty")]
    value: String,
}

#[derive(Deserialize)]
enum DeserializeOnlyEnum {
    #[serde(skip_serializing)]
    Hidden,
}

#[derive(Serialize)]
#[serde(rename(deserialize = "serialize_only_input"))]
struct SerializeOnlyDirectionalRename {
    value: String,
}

#[derive(Deserialize)]
#[serde(rename(serialize = "deserialize_only_output"))]
struct DeserializeOnlyDirectionalRename {
    value: String,
}

#[derive(Serialize)]
struct SerializeOnlyDirectionalField {
    #[serde(rename(deserialize = "field_input"))]
    value: String,
}

#[derive(Deserialize)]
struct DeserializeOnlyDirectionalField {
    #[serde(rename(serialize = "field_output"))]
    value: String,
}

#[derive(Serialize)]
enum SerializeOnlyDirectionalVariant {
    #[serde(rename(deserialize = "variant_input"))]
    Value,
}

#[derive(Deserialize)]
enum DeserializeOnlyDirectionalVariant {
    #[serde(rename(serialize = "variant_output"))]
    Value,
}

#[derive(Serialize)]
#[serde(rename_all(deserialize = "camelCase"))]
struct SerializeOnlyDirectionalRenameAll {
    field_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct DeserializeOnlyDirectionalRenameAll {
    field_name: String,
}

#[derive(Serialize)]
#[serde(rename_all_fields(deserialize = "camelCase"))]
enum SerializeOnlyDirectionalRenameAllFields {
    Fields { field_name: String },
}

#[derive(Deserialize)]
#[serde(rename_all_fields(serialize = "camelCase"))]
enum DeserializeOnlyDirectionalRenameAllFields {
    Fields { field_name: String },
}

#[derive(Serialize)]
#[serde(bound(deserialize = "String: serde::Deserialize<'de>"))]
struct SerializeOnlyDirectionalBound {
    value: String,
}

#[derive(Deserialize)]
#[serde(bound(serialize = "String: serde::Serialize"))]
struct DeserializeOnlyDirectionalBound {
    value: String,
}

#[derive(Serialize)]
struct SerializeOnlyRootEntryAfterActive {
    #[serde(
        skip_serializing_if = "String::is_empty",
        rename(deserialize = "field_input")
    )]
    value: String,
}

#[derive(Deserialize)]
struct DeserializeOnlyRootEntryAfterActive {
    #[serde(default, rename(serialize = "field_output"))]
    value: String,
}

#[derive(Serialize)]
#[cfg_attr(all(), serde(rename(deserialize = "cfg_attr_input")))]
struct CfgAttrDirectionalRename {
    value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename(serialize = "both_output", deserialize = "both_input"))]
struct BothDirectionalRename {
    value: String,
}

#[derive(Serialize)]
struct SerializeOnlyMixedDirectionalField {
    #[serde(
        rename(deserialize = "field_input", serialize = "field_output"),
        skip_serializing_if = "String::is_empty"
    )]
    value: String,
}

#[derive(Deserialize)]
struct DeserializeOnlyMixedDirectionalField {
    #[serde(
        rename(deserialize = "field_input", serialize = "field_output"),
        default
    )]
    value: String,
}

#[derive(Serialize)]
struct DirectionalRenameWithComment {
    #[serde(rename(deserialize = "comment_input", /* keep */ serialize = "comment_output"))]
    value: String,
}

#[derive(Serialize)]
struct DirectionalRenameWithCommentInsideInactiveEntry {
    #[serde(rename(deserialize /* keep */ = "commented_input", serialize = "comment_output"))]
    value: String,
}

#[derive(Serialize)]
struct DirectionalRenameWithCommentInsideOnlyEntry {
    #[serde(rename(deserialize /* keep */ = "commented_input"))]
    value: String,
}

#[derive(Serialize)]
struct DirectionalRenameWithCommentMarkerInString {
    #[serde(rename(deserialize = "/* string value */", serialize = "comment_output"))]
    value: String,
}

#[derive(Serialize)]
struct RenamedToKeyword {
    #[serde(rename = "default")]
    value: String,
}

#[derive(Serialize)]
#[cfg_attr(all(), derive(Deserialize))]
struct CfgAttrBothDirections {
    #[serde(default)]
    value: String,
}

#[derive(Serialize)]
struct MultiEntry {
    #[serde(rename = "v", default)]
    value: String,
}

mod other {
    use serde::Deserialize;

    // Shares a name with the serialize-only struct above but derives both directions.
    #[derive(serde::Serialize, Deserialize)]
    struct SerializeOnly {
        #[serde(alias = "old_value")]
        value: String,
    }
}

#[derive(Serialize, Deserialize)]
struct BothDirections {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    value: String,
}

fn main() {}
