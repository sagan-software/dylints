struct Row;

struct Rows {
    rows: Vec<Row>,
}

type RowVec = Vec<Row>;
type RowIntoIter = std::vec::IntoIter<Row>;

impl Rows {
    fn into_rows(self) -> Vec<Row> {
        self.rows
    }

    fn into_row_vec(self) -> RowVec {
        self.rows
    }

    fn into_row_iter(self) -> RowIntoIter {
        self.rows.into_iter()
    }

    fn iter_rows(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter()
    }

    fn items(self) -> std::vec::IntoIter<Row> {
        self.rows.into_iter()
    }

    fn sorted_rows(self) -> Vec<Row> {
        self.rows
    }

    fn iter_sorted_rows(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter()
    }

    fn iter_rows_with_limit(&self, limit: usize) -> impl Iterator<Item = &Row> {
        self.rows.iter().take(limit)
    }
}

impl IntoIterator for Rows {
    type Item = Row;
    type IntoIter = std::vec::IntoIter<Row>;

    fn into_iter(self) -> Self::IntoIter {
        self.rows.into_iter()
    }
}

struct CustomVec<T>(T);

struct CustomRows {
    row: Row,
}

impl CustomRows {
    fn into_rows(self) -> CustomVec<Row> {
        CustomVec(self.row)
    }
}

struct LookalikeIterator;

struct LookalikeRows;

impl LookalikeRows {
    fn iter_rows(&self) -> LookalikeIterator {
        LookalikeIterator
    }
}

fn main() {}
