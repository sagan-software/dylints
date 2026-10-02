// compile-flags: --edition 2024

fn consume(_: &str) {}

fn main() {
    consume("/home/sagan/balls");
    consume("~/Sync/playground/whatever");
    consume(r"C:\Users\sagan\AppData\Local\example");
    consume(r"\\server\share\example");

    consume("assets/config.toml");
    consume("./assets/config.toml");
    consume("../src/lib.rs");
    consume("https://example.com/assets");
}
