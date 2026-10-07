use std::time::Duration;

async fn bad() {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn bad_while(mut remaining: u8) {
    while remaining > 0 {
        remaining -= 1;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn good() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}

async fn good_spawned_task() {
    for _ in 0..3 {
        drop(tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }));
    }
}

async fn good_nested_function() {
    loop {
        async fn once() {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        once().await;
    }
}

fn main() {}
