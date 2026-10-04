#![allow(dead_code)]

struct Request {
    tenant_id: u64,
    reason_code: String,
    reasons: Vec<String>,
    notes: String,
}

fn http_status() -> Option<u16> {
    Some(202)
}

fn lookup(account_id: Option<u32>) {
    let _ = account_id;
}

fn state_slot(state: &str) -> usize {
    match state {
        "queued" => 0,
        "running" => 1,
        "complete" => 2,
        _ => 3,
    }
}

fn main() {}

struct NestedRequest {
    nested_account_id: MaybeEight<u64>,
    beyond_account_id: Maybe<MaybeEight<u64>>,
    typed_reason_code: Option<ReasonCode>,
}

type Maybe<T> = Option<T>;
type MaybeTwo<T> = Maybe<Maybe<T>>;
type MaybeFour<T> = MaybeTwo<MaybeTwo<T>>;
type MaybeEight<T> = MaybeFour<MaybeFour<T>>;

struct ReasonCode(String);
