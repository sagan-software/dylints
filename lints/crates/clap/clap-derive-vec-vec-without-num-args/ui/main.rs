#![allow(dead_code)]

use clap::Parser;

#[derive(Parser)]
struct MissingBoundary {
    #[arg(long)]
    pairs: Vec<Vec<String>>,
    #[arg(long)]
    optional_pairs: Option<Vec<Vec<String>>>,
}

#[derive(Parser)]
struct PositionalMissingBoundary {
    values: Vec<Vec<String>>,
}

#[derive(Parser)]
struct ExplicitBoundary {
    #[arg(long, num_args = 2)]
    pairs: Vec<Vec<String>>,
    #[arg(long, num_args = 1..)]
    optional_pairs: Option<Vec<Vec<String>>>,
}

fn main() {}
