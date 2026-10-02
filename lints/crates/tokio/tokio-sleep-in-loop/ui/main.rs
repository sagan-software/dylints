use std::time::Duration;

async fn bad() {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn good() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}

fn main() {}
