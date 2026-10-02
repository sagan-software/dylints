fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_prepend_module_to_snapshot(true);
}
