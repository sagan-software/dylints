use sqlx::QueryBuilder;

fn main() {
    let mut query = QueryBuilder;
    query.push_values::<u8>([]);
    query.push_values([1_u8]);
}
