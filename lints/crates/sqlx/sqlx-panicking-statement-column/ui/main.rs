use sqlx::{Statement as _, TestStatement};

fn main() {
    TestStatement.column(0);
    let _ = TestStatement.try_column(0);
}
