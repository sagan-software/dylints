use sqlx::PoolOptions;

fn main() {
    let _ = PoolOptions.max_connections(0);
    let _ = PoolOptions.max_connections(1);
}
