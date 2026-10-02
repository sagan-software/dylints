type MainResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn main() -> MainResult {
    let value: Result<u8, &str> = Ok(1);
    let _value = value.expect("allowed in fallible main");
    Ok(())
}
