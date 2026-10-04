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

struct OptionalProfile {
    country: Option<String>,
    alias_country: Maybe<String>,
    nested_country: MaybeTwo<String>,
    typed_country: Option<CountryCode>,
    at_limit_country: MaybeEight<String>,
    beyond_limit_country: Maybe<MaybeEight<String>>,
    lookalike_country: option_lookalike::Option<String>,
    opaque_country: Option<CountryContainer<String>>,
}

type Maybe<T> = Option<T>;
type MaybeTwo<T> = Maybe<Maybe<T>>;
type MaybeFour<T> = MaybeTwo<MaybeTwo<T>>;
type MaybeEight<T> = MaybeFour<MaybeFour<T>>;

struct CountryContainer<T>(T);

mod option_lookalike {
    pub struct Option<T>(pub T);
}
