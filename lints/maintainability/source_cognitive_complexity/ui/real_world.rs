// compile-flags: --edition 2024

#![allow(dead_code)]

// These reduced examples preserve control-flow shapes found in popular crates.
// Names, types, and side effects are simplified so the fixture uses only std.

// Case 091: reduced from serde_json whitespace scanning.
fn scan_whitespace(bytes: &[u8]) -> usize {
    let mut index = 0;
    while let Some(byte) = bytes.get(index) {
        match byte {
            b' ' | b'\n' | b'\t' | b'\r' => index += 1,
            _ => break,
        }
    }
    index
}

// Case 092: reduced from serde_json token classification.
fn classify_token(byte: Option<u8>) -> &'static str {
    match byte {
        Some(b'n') => "null",
        Some(b't' | b'f') => "boolean",
        Some(b'0'..=b'9' | b'-') => "number",
        Some(b'"') => "string",
        Some(_) => "invalid",
        None => "eof",
    }
}

// Case 093: reduced from tokio task-state transitions.
fn transition_task(is_complete: bool, is_cancelled: bool, is_notified: bool) -> &'static str {
    if is_complete {
        "complete"
    } else if is_cancelled {
        "cancelled"
    } else if is_notified {
        "scheduled"
    } else {
        "idle"
    }
}

// Case 094: reduced from clap value-source selection.
fn select_value_source(
    cli: Option<&str>,
    env: Option<&str>,
    default: Option<&str>,
) -> Option<String> {
    if let Some(value) = cli {
        return Some(value.to_owned());
    }
    if let Some(value) = env {
        return Some(value.to_owned());
    }
    default.map(str::to_owned)
}

// Case 095: reduced from reqwest redirect policy checks.
fn redirect_action(status: u16, previous: usize, limit: usize, same_origin: bool) -> &'static str {
    if !(300..400).contains(&status) {
        return "stop";
    }
    if previous >= limit {
        return "error";
    }
    if same_origin {
        "follow"
    } else {
        "strip-and-follow"
    }
}

// Case 096: reduced from regex literal-prefix scanning.
fn literal_prefix(bytes: &[u8]) -> usize {
    for (index, byte) in bytes.iter().enumerate() {
        if !byte.is_ascii_alphanumeric() && *byte != b'_' {
            return index;
        }
    }
    bytes.len()
}

// Case 097: reduced from tracing interest combination.
fn combine_interest(left: u8, right: u8) -> u8 {
    match (left, right) {
        (2, _) | (_, 2) => 2,
        (1, _) | (_, 1) => 1,
        _ => 0,
    }
}

// Case 098: reduced from rayon split decisions.
fn split_range(start: usize, end: usize, minimum: usize) -> Option<(usize, usize)> {
    let length = end.saturating_sub(start);
    if length <= minimum {
        return None;
    }
    let middle = start + length / 2;
    Some((middle, end))
}

// Case 099: reduced from axum-style method routing.
fn route_method(method: &str, has_body: bool) -> &'static str {
    match method {
        "GET" | "HEAD" => "read",
        "POST" if has_body => "create",
        "PUT" | "PATCH" if has_body => "update",
        "DELETE" => "delete",
        _ => "reject",
    }
}

// Case 100: reduced from anyhow-style source-chain traversal.
fn chain_depth(parents: &[Option<usize>], mut current: usize) -> usize {
    let mut depth = 0;
    while let Some(Some(parent)) = parents.get(current) {
        depth += 1;
        current = *parent;
        if depth == parents.len() {
            break;
        }
    }
    depth
}

fn main() {}
