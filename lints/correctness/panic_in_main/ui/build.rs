fn main() {
    let value: Result<u8, &str> = Ok(1);
    let _value = value.expect("allowed in build script");
}
