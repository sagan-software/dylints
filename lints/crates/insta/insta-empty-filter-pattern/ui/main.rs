fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.add_filter("", "[redacted]");
    settings.add_filter(r"\b[[:xdigit:]]{32}\b", "[id]");
}
