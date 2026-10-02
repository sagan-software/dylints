use sqlx::QueryBuilder;

fn main() {
    let mut query = QueryBuilder;
    query.push(format!("id = {}", 1));
    query.push("id = ");
    query.push(&format!("id = {}", 2));
    query.push(std::format!("id = {}", 3));
    let fixed = format!("id = {}", 4);
    query.push(fixed);
    query.push(format_args!("id = {}", 5));
}
