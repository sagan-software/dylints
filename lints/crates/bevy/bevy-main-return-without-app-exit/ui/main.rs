#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_crate_dependencies,
    unused_results,
    clippy::missing_docs_in_private_items,
    reason = "UI fixture intentionally contains standalone lint examples"
)]

use bevy_app::App;

fn helper() {
    App::new().run();
}

fn helper_with_closure() {
    let _positive_count = [1, 2, 3].into_iter().filter(|value| *value > 0).count();
}

mod nested {
    use bevy_app::App;

    pub fn main() {
        App::new().run();
    }
}

fn main() -> () {
    App::new().run();
    let _ = App::new().run();
    if false {
        App::new().run();
    }
    nested::main();
}
