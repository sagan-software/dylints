#![allow(dead_code)]

async fn clients_in_loops() -> Result<(), reqwest::Error> {
    for _ in 0..2 {
        let _client = reqwest::Client::new();
    }

    for _ in 0..2 {
        let _client = reqwest::Client::builder().build()?;
    }
    Ok(())
}

fn blocking_client_in_loop() {
    while std::hint::black_box(false) {
        let _client = reqwest::blocking::Client::new();
    }
}

async fn valid_client_outside_loop() -> Result<(), reqwest::Error> {
    let client = reqwest::Client::builder().build()?;
    for _ in 0..2 {
        let _response = client.get("https://example.com").send().await?;
    }
    Ok(())
}

fn main() {}
