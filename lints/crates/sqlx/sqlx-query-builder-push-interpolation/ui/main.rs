use sqlx::QueryBuilder;

fn main() {
    let mut query = QueryBuilder;
    query.push(format!("id = {}", 1));
    query.push("id = ");
}
