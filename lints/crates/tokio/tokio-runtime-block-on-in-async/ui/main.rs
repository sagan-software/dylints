use tokio::runtime::Runtime;

async fn bad(runtime: &Runtime) {
    runtime.block_on(async {});
    let task = async { 1 };
    let _ = runtime.block_on(task);
}

fn good(runtime: &Runtime) {
    runtime.block_on(async {});
}

async fn good_sync_closure(runtime: &Runtime) {
    let run = || runtime.block_on(async {});
    drop(run);
}

fn main() {}
