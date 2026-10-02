fn main() {
    let _ = tokio::task::spawn_blocking(|| async {});
    let _ = tokio::task::spawn_blocking(|| {});
}
