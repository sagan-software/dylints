use tokio::runtime::Runtime;

async fn bad(runtime: &Runtime) {
    runtime.block_on(async {});
}

fn good(runtime: &Runtime) {
    runtime.block_on(async {});
}

fn main() {}
