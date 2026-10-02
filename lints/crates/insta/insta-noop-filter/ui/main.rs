// run-rustfix
// rustfix-only-machine-applicable
const TOKEN: &str = "token";

fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.add_filter("token", "token");
    settings.add_filter(TOKEN, "token");
    let () = settings.add_filter("token", "token");
    settings.add_filter("token", "[token]");
    settings.add_filter("a.b", "a.b");
}
