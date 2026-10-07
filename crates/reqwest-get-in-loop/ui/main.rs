#![allow(dead_code)]

async fn shortcut_in_loop() -> Result<(), reqwest::Error> {
    for _ in 0..2 {
        let _response = reqwest::get("https://example.com").await?;
    }
    Ok(())
}

fn blocking_shortcut_in_loop() -> Result<(), reqwest::Error> {
    for _ in 0..2 {
        let _response = reqwest::blocking::get("https://example.com")?;
    }
    Ok(())
}

async fn valid_reused_client() -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    for _ in 0..2 {
        let _response = client.get("https://example.com").send().await?;
    }
    Ok(())
}

async fn valid_single_shortcut() -> Result<(), reqwest::Error> {
    let _response = reqwest::get("https://example.com").await?;
    Ok(())
}

fn main() {}
