use sqlx::{Dynamic, OverriddenRow, Row as _, Statement as _, TestRow, TestStatement};

struct LocalColumns;

impl LocalColumns {
    fn column(&self, _index: usize) {}
}

fn main() {
    TestStatement.column(0);
    let _ = TestStatement.try_column(0);
    TestRow.column("name");
    let _ = TestRow.try_column("name");
    OverriddenRow.column(0);
    LocalColumns.column(0);
}

fn dynamic_column(value: &dyn Dynamic) {
    value.column(0);
}
