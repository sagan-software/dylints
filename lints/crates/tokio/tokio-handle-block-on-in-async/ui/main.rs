use std::future::{Ready, ready};
use tokio::runtime::Handle;

async fn bad(handle: &Handle, future: Ready<u8>) {
    handle.block_on(async {});
    let _ = handle.block_on(future);
    let _ = handle.block_on(if true { ready(1) } else { ready(2) });
}

fn good(handle: &Handle) {
    handle.block_on(async {});
}

async fn good_nested(handle: &Handle) {
    fn nested(handle: &Handle) {
        handle.block_on(async {});
    }
    nested(handle);
}

fn main() {}
