use sqlx::QueryBuilder;

fn main() {
    let mut query = QueryBuilder;
    let local_empty_values: fn() -> Vec<u8> = Vec::new;
    query.push_values::<&u8>(&[]);
    query.push_values(Vec::<u8>::new());
    query.push_values(vec![0_u8; 0]);
    query.push_values::<u8>(vec![]);
    query.push_values(std::iter::empty::<u8>());
    query.push_values::<u8>([]);
    query.push_values((|| Vec::<u8>::new())());
    query.push_values(local_empty_values());
    query.push_values([1_u8]);
}
