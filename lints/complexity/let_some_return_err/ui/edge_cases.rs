#![allow(dead_code)]

#[derive(Debug)]
enum Error {
    Missing,
}

fn lookup_pair() -> Option<(u64, &'static str)> {
    Some((7, "seven"))
}

fn destructured_return() -> Result<u64, Error> {
    let Some((id, _name)) = lookup_pair() else {
        return Err(Error::Missing);
    };
    Ok(id)
}

fn allowed_continue(items: &[Option<u64>]) -> u64 {
    let mut sum = 0;
    for item in items {
        let Some(value) = item else {
            continue;
        };
        sum += value;
    }
    sum
}

fn main() {}
