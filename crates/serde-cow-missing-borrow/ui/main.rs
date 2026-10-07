#![allow(dead_code)]

use std::borrow::Cow as TextCow;
use std::{borrow::Cow, marker::PhantomData};

use serde::Deserialize;

type BorrowedText<'a> = std::borrow::Cow<'a, str>;

mod local {
    use super::PhantomData;

    pub struct Cow<'a, T: ?Sized>(PhantomData<&'a T>);

    impl<'de, T: ?Sized> serde::Deserialize<'de> for Cow<'_, T> {
        fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            unimplemented!()
        }
    }
}

#[cfg(any())]
#[derive(Deserialize)]
struct CfgDisabled<'a> {
    body: Cow<'a, str>,
}

#[derive(Deserialize)]
struct Comment<'a> {
    body: Cow<'a, str>,
    raw: Cow<'a, [u8]>,
    qualified: std::borrow::Cow<'a, str>,
    renamed: TextCow<'a, str>,
    aliased: BorrowedText<'a>,
}

#[derive(Deserialize)]
struct BorrowedComment<'a> {
    #[serde(borrow)]
    body: Cow<'a, str>,
}

#[derive(Deserialize)]
struct StaticComment {
    body: Cow<'static, str>,
}

#[derive(Deserialize)]
enum Event<'a> {
    Note { body: Cow<'a, str> },
}

#[derive(serde::Serialize)]
struct SerializeOnly<'a> {
    body: Cow<'a, str>,
}

#[derive(serde::Deserialize)]
struct LocalLookalike<'a> {
    body: local::Cow<'a, str>,
}

macro_rules! derived_comment {
    ($name:ident) => {
        #[derive(Default, Deserialize)]
        struct $name<'a> {
            #[serde(borrow)]
            body: Cow<'a, str>,
        }
    };
}

derived_comment!(MacroComment);

macro_rules! inner_default {
    ($name:ident) => {
        impl Default for $name {
            fn default() -> Self {
                Self
            }
        }
    };
}

macro_rules! outer_default {
    ($name:ident) => {
        inner_default!($name);
    };
}

struct NestedDefault;

outer_default!(NestedDefault);

#[derive(Deserialize)]
struct NonCowInputs<'a> {
    borrowed: &'a str,
    number: u32,
}

fn main() {}
