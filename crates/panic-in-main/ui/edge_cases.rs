#![allow(dead_code)]

fn main() {
    let value = Some("config").expect("config must be present");
    assert!(!value.is_empty());
}

fn result_main() -> Result<(), &'static str> {
    let _value = Some("config").expect("allowed outside infallible main");
    Ok(())
}

fn helper() {
    todo!("allowed outside main");
}
