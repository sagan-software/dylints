fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.set_allow_empty_glob(true);
    settings.set_allow_empty_glob(false);
}
