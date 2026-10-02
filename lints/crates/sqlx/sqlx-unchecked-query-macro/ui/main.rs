#![allow(dead_code)]

fn invalid_unchecked_queries() {
    let _ = sqlx::query_unchecked!("SELECT 1");
    let _ = sqlx::query_as_unchecked!(u64, "SELECT 1");
    let _ = sqlx::query_file_unchecked!("queries/one.sql");
    let _ = sqlx::query_file_as_unchecked!(u64, "queries/one.sql");
    let _ = sqlx::query_scalar_unchecked!("SELECT 1");
    let _ = sqlx::query_file_scalar_unchecked!("queries/one.sql");
}

fn valid_checked_queries() {
    let _ = sqlx::query!("SELECT 1");
    let _ = sqlx::query_as!(u64, "SELECT 1");
    let _ = sqlx::query_file!("queries/one.sql");
    let _ = sqlx::query_file_as!(u64, "queries/one.sql");
    let _ = sqlx::query_scalar!("SELECT 1");
    let _ = sqlx::query_file_scalar!("queries/one.sql");
}

macro_rules! query_unchecked {
    ($query:literal) => {
        $query
    };
}

fn similarly_named_local_macro() {
    let _ = query_unchecked!("SELECT 1");
}

fn main() {}
