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

#[derive(Parser)]
struct OtherFields {
    #[arg(long)]
    single: Vec<String>,
    #[arg(skip)]
    skipped_groups: Vec<Vec<String>>,
    #[arg(skip)]
    boxed: Box<u8>,
    #[arg(skip)]
    pair: (u8, u8),
    #[arg(skip)]
    qualified: std::string::String,
    #[arg(skip)]
    map: std::collections::HashMap<String, String>,
}

#[derive(Clone, clap::ValueEnum)]
enum Mode {
    Fast,
}

fn main() {}
