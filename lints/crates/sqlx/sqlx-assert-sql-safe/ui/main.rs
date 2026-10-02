#![allow(dead_code)]

use sqlx::AssertSqlSafe;

fn invalid_dynamic_sql(table: &str) {
    let _query = AssertSqlSafe(format!("SELECT * FROM {table}"));
}

fn invalid_manual_audit(query: String) {
    let _query = sqlx::AssertSqlSafe(query);
}

fn valid_static_sql() {
    let _query = "SELECT * FROM users WHERE id = $1";
}

struct OtherAssertion<T>(T);

fn similarly_named_user_type(query: String) {
    let _query = OtherAssertion(query);
}

fn main() {}
