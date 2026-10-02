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
    #[serde(borrow)]
    aliased_name: BorrowedStr<'a>,
    #[serde(borrow)]
    aliased_raw: BorrowedBytes<'a>,
    #[serde(borrow)]
    path: BorrowedPath<'a>,
    #[serde(borrow)]
    inline_name: &'a str,
}

#[derive(Deserialize)]
struct WithCow<'a> {
    #[serde(borrow)]
    name: std::borrow::Cow<'a, str>,
}

fn main() {}
