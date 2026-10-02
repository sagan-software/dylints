fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.add_filter("token", "token");
    settings.add_filter("token", "[token]");
}
