use sqlx::{Row as _, Statement as _, TestRow, TestStatement};

struct LocalColumns;

impl LocalColumns {
    fn column(&self, _index: usize) {}
}

fn main() {
    TestStatement.column(0);
    let _ = TestStatement.try_column(0);
    TestRow.column("name");
    let _ = TestRow.try_column("name");
    LocalColumns.column(0);
}
