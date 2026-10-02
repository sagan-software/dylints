fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path("");
    settings.set_snapshot_path("API response");
}
