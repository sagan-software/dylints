#![feature(rustc_private)]

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if let Err(error) = dylint_driver::dylint_driver(&args) {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
