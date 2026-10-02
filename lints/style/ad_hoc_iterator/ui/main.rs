struct Row;

struct Rows {
    rows: Vec<Row>,
}

type MaybeRow = Option<Row>;

impl Rows {
    fn next_row(&mut self) -> Option<Row> {
        self.rows.pop()
    }

    fn next_alias(&mut self) -> MaybeRow {
        self.rows.pop()
    }

    fn next_qualified(&mut self) -> std::option::Option<Row> {
        self.rows.pop()
    }

    fn next_filtered(&mut self, _include_archived: bool) -> Option<Row> {
        self.rows.pop()
    }

    fn next_page(&mut self) -> Option<Row> {
        self.rows.pop()
    }

    fn peek(&self) -> Option<&Row> {
        self.rows.last()
    }
}

impl std::iter::Iterator for Rows {
    type Item = Row;

    fn next(&mut self) -> Option<Self::Item> {
        self.rows.pop()
    }
}

mod local {
    pub struct Option<T>(pub std::marker::PhantomData<T>);

    pub trait Iterator {
        type Item;

        fn next(&mut self) -> std::option::Option<Self::Item>;
    }
}

struct Lookalike;

impl Lookalike {
    fn next_row(&mut self) -> local::Option<Row> {
        local::Option(std::marker::PhantomData)
    }
}

impl local::Iterator for Lookalike {
    type Item = Row;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

struct State;

impl State {
    fn next_state(&mut self) -> Option<Self> {
        None
    }
}

fn main() {}
