use tokio::runtime::Runtime;

async fn bad() {
    let _ = Runtime::new();
}

fn good() {
    let _ = Runtime::new();
}

fn main() {}
