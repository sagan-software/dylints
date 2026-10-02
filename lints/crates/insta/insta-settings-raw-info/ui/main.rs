fn main() {
    let mut settings = insta::Settings::clone_current();
    let content = insta::internals::Content::String("secret".into());
    settings.set_raw_info(&content);
}
