fn main() {
    let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let (_tx, _rx) = tokio::sync::mpsc::channel::<u8>(8);
}
