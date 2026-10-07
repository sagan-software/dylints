#![allow(dead_code)]

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author)]
struct HiddenAuthor {}

#[derive(Parser)]
#[command(author, help_template = "{about}\n{author}\n{usage}\n{options}")]
struct VisibleAuthor {}

#[derive(Subcommand)]
enum Commands {
    #[command(author = "Example Team")]
    Hidden {},
    #[command(
        author = "Example Team",
        help_template = "{about}\n{author}\n{usage}\n{options}"
    )]
    Visible {},
}

fn main() {}
