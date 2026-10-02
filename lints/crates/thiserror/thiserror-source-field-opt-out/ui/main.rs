use thiserror::Error;

#[cfg(any())]
#[derive(Error, Debug)]
#[error("{source} -> {destination}")]
pub struct RouteError {
    source: char,
    destination: char,
}

#[derive(Error, Debug)]
#[error("{source} -> {destination}")]
pub struct RawSourceRouteError {
    r#source: char,
    destination: char,
}

#[derive(Error, Debug)]
#[error("io")]
pub struct RealSource {
    source: std::io::Error,
}

fn main() {}
