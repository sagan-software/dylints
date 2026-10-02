#![allow(dead_code)]

use sqlx::PoolConnection;

fn invalid_permanent_checkout(connection: PoolConnection) {
    let _connection = connection.leak();
}

fn valid_detach(connection: PoolConnection) {
    let _connection = connection.detach();
}

struct OtherConnection;

impl OtherConnection {
    fn leak(self) {}
}

fn similarly_named_user_method(connection: OtherConnection) {
    connection.leak();
}

fn main() {}
