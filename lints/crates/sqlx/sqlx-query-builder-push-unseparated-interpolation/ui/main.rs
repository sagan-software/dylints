use sqlx::QueryBuilder;

fn main() {
    let mut query = QueryBuilder;
    let mut separated = query.separated(", ");
    separated.push_unseparated(format!("id = {}", 1));
    separated.push_unseparated("id = ");
}
