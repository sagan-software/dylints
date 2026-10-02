#![allow(dead_code)]

fn manual_content_types(client: &reqwest::Client) {
    let form = reqwest::multipart::Form::new().text("name", "value");
    let _request = client
        .post("https://example.com")
        .header(reqwest::header::CONTENT_TYPE, "multipart/form-data")
        .multipart(form);

    let form = reqwest::multipart::Form::new().text("name", "value");
    let _request = client
        .post("https://example.com")
        .multipart(form)
        .header("content-type", "multipart/form-data");
}

fn valid_multipart(client: &reqwest::Client) {
    let form = reqwest::multipart::Form::new().text("name", "value");
    let _request = client
        .post("https://example.com")
        .header(reqwest::header::ACCEPT, "application/json")
        .multipart(form);
}

fn main() {}
