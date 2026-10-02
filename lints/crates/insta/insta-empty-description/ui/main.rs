const EMPTY: &str = "";

fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.set_description("");
    settings.set_description(EMPTY);
    settings.set_description("API response");
}
