#![allow(dead_code)]

async fn blocking_calls() -> Result<(), reqwest::Error> {
    let _response = reqwest::blocking::get("https://example.com")?;

    let client = reqwest::blocking::Client::new();
    let _response = client.get("https://example.com").send()?;
    Ok(())
}

fn valid_synchronous_call() -> Result<(), reqwest::Error> {
    let _response = reqwest::blocking::get("https://example.com")?;
    Ok(())
}

async fn valid_async_call() -> Result<(), reqwest::Error> {
    let _response = reqwest::get("https://example.com").await?;
    Ok(())
}

fn main() {}
