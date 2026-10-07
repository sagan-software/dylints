fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let value: Result<u8, &str> = Ok(1);
    let _value = value.expect("allowed in fallible main");
    Ok(())
}
