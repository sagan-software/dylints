use tokio::runtime::Handle;

async fn bad(handle: &Handle) {
    handle.block_on(async {});
}

fn good(handle: &Handle) {
    handle.block_on(async {});
}

fn main() {}
