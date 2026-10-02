#![allow(dead_code)]

use clap::{ArgAction, Parser};

#[derive(Parser)]
struct IneffectiveFlags {
    #[arg(long, default_value_t = true)]
    color: bool,
    #[arg(long, default_value = "true")]
    progress: bool,
}

#[derive(Parser)]
struct EffectiveFlags {
    #[arg(long)]
    color: bool,
    #[arg(long, default_value_t = false)]
    progress: bool,
    #[arg(long = "no-cache", action = ArgAction::SetFalse, default_value_t = true)]
    cache: bool,
}

fn main() {}
