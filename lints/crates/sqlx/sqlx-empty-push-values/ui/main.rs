use sqlx::QueryBuilder;

fn main() {
    let mut query = QueryBuilder;
    query.push_values::<&u8>(&[]);
    query.push_values(Vec::<u8>::new());
    query.push_values(vec![0_u8; 0]);
    query.push_values::<u8>(vec![]);
    query.push_values(std::iter::empty::<u8>());
    query.push_values::<u8>([]);
    query.push_values([1_u8]);
}
