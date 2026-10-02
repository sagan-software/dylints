use std::env;
use std::env::args;
use std::env::args_os as os_args;

fn local_args() {
    fn args() -> Vec<String> {
        Vec::new()
    }

    let _ = args();
}

mod local_env {
    pub fn args() -> Vec<String> {
        Vec::new()
    }
}

fn parses_manually() {
    let _ = std::env::args().nth(1);
    let _ = std::env::args_os().nth(1);
    let _ = env::args().skip(1);
    let _ = env::args_os().skip(1);
    let _ = args().skip(1);
    let _ = os_args().skip(1);
}

fn allowed_calls() {
    let _ = local_env::args();
    let _ = local_args as fn();
}

fn main() {}

// Local callable bindings are not resolved standard-library function calls.
fn callable_bindings() {
    let args = || Vec::<String>::new();
    let _ = args();
    let args_fn = std::env::args;
    let _ = args_fn();
}
