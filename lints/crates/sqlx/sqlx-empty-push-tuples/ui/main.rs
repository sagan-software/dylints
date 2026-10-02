use sqlx::QueryBuilder;

fn main() {
    let mut query = QueryBuilder;
    query.push_tuples::<u8>([]);
    query.push_tuples([1_u8]);
}
