struct Row;

struct Rows {
    rows: Vec<Row>,
}

type RowVec = Vec<Row>;
type RowIntoIter = std::vec::IntoIter<Row>;

impl Rows {
    const LIMIT: usize = 10;

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

    fn iter_slice(&self) -> std::slice::Iter<'_, Row> {
        self.rows.iter()
    }

    fn items(self) -> std::vec::IntoIter<Row> {
        self.rows.into_iter()
    }

    fn into_bytes(self) -> Vec<u8> {
        Vec::new()
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

    fn iter_from(rows: Vec<Row>) -> std::vec::IntoIter<Row> {
        rows.into_iter()
    }
}

struct ImplementedRows {
    rows: Vec<Row>,
}

impl ImplementedRows {
    fn into_rows(self) -> std::vec::IntoIter<Row> {
        self.rows.into_iter()
    }

    fn iter_rows(&self) -> std::slice::Iter<'_, Row> {
        self.rows.iter()
    }
}

impl IntoIterator for ImplementedRows {
    type Item = Row;
    type IntoIter = std::vec::IntoIter<Row>;

    fn into_iter(self) -> Self::IntoIter {
        self.rows.into_iter()
    }
}

impl<'a> IntoIterator for &'a ImplementedRows {
    type Item = &'a Row;
    type IntoIter = std::slice::Iter<'a, Row>;

    fn into_iter(self) -> Self::IntoIter {
        self.rows.iter()
    }
}

trait RowSource {
    fn iter_source(&self) -> std::slice::Iter<'_, Row>;
}

impl RowSource for Rows {
    fn iter_source(&self) -> std::slice::Iter<'_, Row> {
        self.rows.iter()
    }
}

struct GenericRows<T>(Vec<T>);

impl<T> GenericRows<T> {
    fn into_values(self) -> std::vec::IntoIter<T> {
        self.0.into_iter()
    }
}

fn main() {}
