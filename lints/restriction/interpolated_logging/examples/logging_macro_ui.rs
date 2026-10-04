//! Calls actual logging macros to check message and field parsing.

#![feature(rustc_private)]

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use interpolated_logging as _;

fn main() {
    let user = 7_u32;
    tracing_messages(user);
    tracing_event(user);
    log_key_value_messages(user);
    log_logger_messages(user);
    concat_messages(user);
    empty_concat_message();
}

/// Check tracing options, fields, and direct message strings.
fn tracing_messages(user: u32) {
    tracing::info!(payload = "{json}", "received",);
    tracing::info!({ payload = "{json}" }, "received");
    tracing::info!(payload = "{json}");
    tracing::info!(user, "received {user}");
    tracing::info!("received {user}");
    tracing::info!(r"received {user}");
    tracing::info!(concat!("received {", "}",), user);
}

/// Check the positional level and structured fields in `tracing::event!`.
fn tracing_event(user: u32) {
    tracing::event!(
        name: "event {user}",
        target: "target.{user}",
        parent: None,
        tracing::Level::INFO,
        payload = format!("payload {user}"),
        "received"
    );
}

/// Check the `log` key/value separator and format arguments.
fn log_key_value_messages(user: u32) {
    log::info!(
        target: concat!("log-target.", "{user}"),
        payload = "{json}";
        "received"
    );
    log::info!(
        target: "log-target",
        payload = "{json}";
        "log received {}",
        user.saturating_add(1),
    );
}

/// Check custom logger and target options in `log` calls.
fn log_logger_messages(user: u32) {
    log::info!(
        logger: log::__private_api::GlobalLogger,
        target: concat!("logger-target.", "{user}"),
        "logger received {user}"
    );
    log::info!(target: "log-target", "log received {}", user.saturating_add(1));
}

/// Check built-in `concat!` literal decoding and interpolation detection.
fn concat_messages(user: u32) {
    tracing::info!(
        concat!(r"received {}", 1, 'x', true, false, 2.5, -3, -0.5),
        user,
    );
    tracing::info!(concat!("{", '0', "}"), user);
    tracing::warn!("\x7b0\x7d", user);
    tracing::warn!(concat!('\x7b', "0}"), user);
    tracing::warn!(concat!("\x7b", "0}"), user);
    tracing::warn!(concat!("\x7b\x7b", "\x7d\x7d"));
}

/// Check that an empty `concat!` message emits no interpolation warning.
#[expect(
    clippy::useless_concat,
    reason = "Exercises an empty concat logging message"
)]
fn empty_concat_message() {
    tracing::info!(concat!());
}
