// compile-flags: --edition=2024

#![allow(dead_code)]

trait AsyncReader {
    async fn read(&self) -> Result<(), String>;
}

fn main() {}
