// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::path::Path;

use serde::Deserialize;

type BorrowedStr<'a> = &'a str;
type BorrowedBytes<'a> = &'a [u8];
type BorrowedPath<'a> = &'a Path;

#[cfg(any())]
#[derive(Deserialize)]
struct CfgDisabled<'a> {
    #[serde(borrow)]
    name: &'a str,
}

#[derive(serde::Deserialize)]
struct User<'a> {
    #[serde(borrow)]
    name: &'a str,
    #[serde(borrow)]
    raw: &'a [u8],
    // Serde does not borrow through an alias, so the attribute is required here.
    #[serde(borrow)]
    aliased_name: BorrowedStr<'a>,
    #[serde(borrow)]
    aliased_raw: BorrowedBytes<'a>,
    #[serde(borrow)]
    optional_name: Option<&'a str>,
    #[serde(borrow)]
    path: BorrowedPath<'a>,
    #[serde(borrow)]
    inline_name: &'a str,
    #[serde(borrow, rename = "renamed")]
    renamed_name: &'a str,
}

#[derive(Deserialize)]
enum Message<'a> {
    Text(#[serde(borrow)] &'a str),
    Owned(String),
}

#[derive(serde::Serialize)]
struct SerializeOnly<'a> {
    #[serde(borrow)]
    name: &'a str,
}

#[derive(Deserialize)]
struct WithCow<'a> {
    #[serde(borrow)]
    name: std::borrow::Cow<'a, str>,
}

fn main() {}
