// compile-flags: --edition 2024

fn consume(_: &str) {}

fn main() {
    consume("/home/sagan/balls");
    consume("~/Sync/playground/whatever");
    consume(r"C:\Users\sagan\AppData\Local\example");
    consume(r"\\server\share\example");
    consume("/Users/alice/Library/Caches");
    consume("/opt");
    consume("~");
    consume("~alice/cache");
    consume(r"~\AppData");

    consume("assets/config.toml");
    consume("./assets/config.toml");
    consume("../src/lib.rs");
    consume("https://example.com/assets");
    consume("/api/users");
    consume("/users/:id");
    consume("// comment text");
    consume("/");
    consume("~tilde-without-separator");
    consume("C:relative");
    let _ = b"/home/sagan/bytes";
    let _ = 7;
}
