#![allow(dead_code)]

use std::string::String as StdString;

struct CountryCode(String);
type CountryText = std::string::String;
type CountryStr = str;
struct CountryTextWrapper(String);

struct Address<'a> {
    country: &'a str,
    shipping_country_code: CountryText,
    iso_country: std::string::String,
    residence_country: &'a CountryStr,
    billing_country: StdString,
    country_name: String,
    typed_country: CountryCode,
    wrapped_country: CountryTextWrapper,
}

fn main() {}
