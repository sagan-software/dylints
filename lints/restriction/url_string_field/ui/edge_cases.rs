#![allow(dead_code)]

use std::string::String as StdString;

struct Url;
type UrlText = std::string::String;
type UrlStr = str;
struct UrlTextWrapper(String);

struct Endpoints<'a> {
    callback_url: &'a UrlStr,
    webhook_uri: UrlText,
    base_endpoint: std::string::String,
    link_url: StdString,
    url_label: String,
    link_text: String,
    endpoint_template: String,
    typed_url: Url,
    typed_endpoint: UrlTextWrapper,
}

fn main() {}
