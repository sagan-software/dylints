struct CountryCode(String);

struct Profile<'a> {
    country: String,
    country_code: &'a str,
    iso_country: String,
    iso_country_code: String,
    residence_country: &'a str,
    nationality_country: String,
    billing_country: String,
    shipping_country_code: &'a str,
    display_name: String,
    country_name: String,
    country_display_name: String,
    non_country_string: String,
    semantic_country: CountryCode,
    semantic_country_code: CountryCode,
}

fn main() {}
