#![allow(dead_code)]

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(verbatim_doc_comment)]
struct MissingDocs {
    #[arg(long, verbatim_doc_comment)]
    output: String,
}

/// Keep this line.
///
/// Keep this second paragraph.
#[derive(Parser)]
#[command(verbatim_doc_comment)]
struct Documented {
    /// Keep the field's line breaks.
    #[arg(long, verbatim_doc_comment)]
    output: String,
}

#[derive(Subcommand)]
enum Commands {
    #[command(verbatim_doc_comment)]
    Run {},
    /// Keep this variant's layout.
    #[command(verbatim_doc_comment)]
    Test {},
    Build {
        #[arg(long, verbatim_doc_comment)]
        target: String,
    },
}

fn main() {}
