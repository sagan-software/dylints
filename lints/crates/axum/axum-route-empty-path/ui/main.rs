// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use axum::{Router, routing::get};

fn routes() {
    let _: Router = Router::new().route("", get(|| async {}));
    let _: Router = Router::new().route("/", get(|| async {}));
}

fn main() {}

fn local_empty_path() {
    let path = "";
    let _: Router = Router::new().route(path, get(|| async {}));
    let _: Router = Router::new().route(path, get(|| async {}));
}

fn local_empty_path_controls() {
    let path0 = "";
    let path1 = path0;
    let path2 = path1;
    let path3 = path2;
    let path4 = path3;
    let path5 = path4;
    let path6 = path5;
    let path7 = path6;
    let _: Router = Router::new().route(path7, get(|| async {}));

    let path0 = "";
    let path1 = path0;
    let path2 = path1;
    let path3 = path2;
    let path4 = path3;
    let path5 = path4;
    let path6 = path5;
    let path7 = path6;
    let path8 = path7;
    let _: Router = Router::new().route(path8, get(|| async {}));
}

fn mutable_and_unknown_paths_stay_unchecked() {
    let mut path = "";
    let _: Router = Router::new().route(path, get(|| async {}));
    path = "/";
    let _: Router = Router::new().route(path, get(|| async {}));
    let path = "/";
    let _: Router = Router::new().route(path, get(|| async {}));
    {
        let path = "";
        let _: Router = Router::new().route(path, get(|| async {}));
    }
    let _: Router = Router::new().route(path, get(|| async {}));
    let (_, path) = ((), "");
    let _: Router = Router::new().route(path, get(|| async {}));
    let _: Router = Router::new().route(unknown_path(), get(|| async {}));
}

fn unknown_path() -> &'static str {
    ""
}
