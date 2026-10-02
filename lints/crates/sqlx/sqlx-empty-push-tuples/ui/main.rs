use sqlx::QueryBuilder;

fn main() {
    let mut query = QueryBuilder;
    query.push_tuples::<&u8>(&[]);
    query.push_tuples(Vec::<u8>::new());
    query.push_tuples(vec![0_u8; 0]);
    query.push_tuples::<u8>(vec![]);
    query.push_tuples(std::iter::empty::<u8>());
    query.push_tuples::<u8>([]);
    query.push_tuples([1_u8]);
}
