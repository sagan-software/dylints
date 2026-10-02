#![allow(dead_code)]

use sqlx::{Row, TestRow};

fn invalid_panicking_access(row: &TestRow) {
    let _: String = row.get("name");
    let _: i64 = row.get_unchecked(0);
}

fn valid_fallible_access(row: &TestRow) {
    let _name: Result<String, _> = row.try_get("name");
    let _id: Result<i64, _> = row.try_get_unchecked(0);
}

struct OtherRow;

impl OtherRow {
    fn get(&self, _name: &str) -> String {
        String::new()
    }
}

fn similarly_named_user_method(row: &OtherRow) {
    let _name = row.get("name");
}

fn main() {}
