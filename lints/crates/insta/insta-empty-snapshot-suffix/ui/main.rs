fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_suffix("");
    settings.set_snapshot_suffix("API response");
}
