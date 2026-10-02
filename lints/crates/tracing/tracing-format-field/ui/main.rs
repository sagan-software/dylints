#![allow(dead_code)]

use tracing::{debug_span, info};

fn invalid_format_fields(request_id: u64, status: &str) {
    info!(
        request = format!("{request_id}:{status}"),
        label = format!("request-{request_id}")
    );
}

fn valid_fields(request_id: u64, status: &str) {
    info!(request_id, %status);
    debug_span!("request", request = %request_id);
    info!("request {request_id}:{status}");
}

fn valid_similarly_named_macro(request_id: u64) {
    macro_rules! format {
        ($value:expr) => {
            String::from($value)
        };
    }
    info!(label = format!("request"), request_id);
}

fn main() {}
