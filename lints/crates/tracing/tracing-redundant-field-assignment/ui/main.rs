#![allow(dead_code)]

use tracing::{debug_span, info};

#[derive(Debug)]
struct User {
    id: u64,
}

fn invalid_assignments(request_id: u64, user: User) {
    info!(request_id = request_id, user.id = user.id);
    debug_span!("request", request_id = %request_id, user = ?user);
}

fn valid_assignments(request_id: u64, user: User) {
    info!(request_id, user.id);
    debug_span!("request", %request_id, ?user);
    info!(request.id = request_id);
}

fn main() {}
