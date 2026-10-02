#![allow(dead_code)]

struct Request {
    id: String,
    label: String,
    owner: String,
    count: usize,
}

struct Response {
    id: String,
    label: String,
    owner: String,
}

fn clones_three_fields(request: Request) -> Response {
    Response {
        id: request.id.clone(),
        label: request.label.clone(),
        owner: request.owner.clone(),
    }
}

fn consumes_fields(request: Request) -> Response {
    Response {
        id: request.id,
        label: request.label,
        owner: request.owner,
    }
}

fn main() {}
