#![allow(dead_code)]

use tracing::{Level, debug, error, event, info};

fn invalid_interpolation(request_id: u64, error: impl std::fmt::Display) {
    info!("request {request_id} completed");
    error!("request {} failed: {}", request_id, error);
    event!(Level::DEBUG, "request {request_id:?} queued");
}

fn valid_structured_fields(request_id: u64, error: impl std::fmt::Display) {
    info!(request_id, "request completed");
    error!(request_id, %error, "request {request_id} failed: {error}");
    debug!(answer = 42, "the answer is {}", 42);
    info!("static message");
}

mod similarly_named_macro {
    macro_rules! info {
        ($message:literal) => {
            let _message = $message;
        };
    }

    pub(super) fn valid_local_macro(request_id: u64) {
        info!("request {request_id} completed");
    }
}

fn main() {}
