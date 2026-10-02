// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
struct RedundantActions {
    #[arg(long, action = ArgAction::SetTrue)]
    verbose: bool,
    #[arg(long, action = clap::ArgAction::Set)]
    output: Option<String>,
    #[arg(long, action = ArgAction::Append)]
    include: Vec<String>,
    #[arg(action = ArgAction::SetTrue)]
    standalone: bool,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(long, action = ArgAction::Set)]
        profile: String,
    },
}

#[derive(Parser)]
struct EntryPositions {
    #[arg(action = ArgAction::SetTrue, long)]
    first: bool,
    #[arg(long, action = ArgAction::SetTrue,)]
    trailing: bool,
    #[arg(
        long,
        action = ArgAction::Set,
        help = "a, b"
    )]
    middle: Option<String>,
}

#[derive(Parser)]
struct CustomActions {
    #[arg(long, action = ArgAction::Count)]
    verbose: u8,
    #[arg(long = "no-color", action = ArgAction::SetFalse)]
    color: bool,
}

#[derive(Parser)]
struct Wrappers {
    #[arg(long, action = ArgAction::Append)]
    optional_values: Option<Vec<String>>,
    #[arg(long, action = ArgAction::Set, help = "say \"hi\", then go")]
    escaped: String,
    #[command(flatten)]
    shared: Shared,
}

#[derive(Args)]
struct Shared {
    #[arg(long)]
    quiet: bool,
}

#[derive(Clone, ValueEnum)]
enum Mode {
    Fast,
}

#[cfg_attr(all(), derive(Parser))]
struct CfgDerived {
    #[arg(long, action = ArgAction::SetTrue)]
    verbose: bool,
}

#[cfg_attr(all(), derive(Subcommand))]
enum CfgCommands {
    #[command(about = "run")]
    Run {
        #[arg(long, action = ArgAction::Set)]
        profile: String,
    },
}

macro_rules! generated_parser {
    () => {
        #[derive(Parser)]
        struct Generated {
            #[arg(long, action = ArgAction::SetTrue)]
            verbose: bool,
        }
    };
}

generated_parser!();

fn main() {}
