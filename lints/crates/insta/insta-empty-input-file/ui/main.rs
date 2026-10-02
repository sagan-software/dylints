fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.set_input_file("");
    settings.set_input_file("API response");
}
