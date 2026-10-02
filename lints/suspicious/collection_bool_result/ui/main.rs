#![allow(dead_code)]

struct Action;
struct Error;

fn classify() -> Result<(Vec<Action>, bool), Error> {
    Ok((Vec::new(), false))
}

fn main() {}
