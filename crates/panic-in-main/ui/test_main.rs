// compile-flags: --test

#[test]
fn main() {
    let value: Result<u8, &str> = Ok(1);
    let _value = value.expect("allowed in test function");
}
