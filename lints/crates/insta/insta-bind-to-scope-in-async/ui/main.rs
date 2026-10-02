async fn bad(settings: &insta::Settings) {
    let _guard = settings.bind_to_scope();
}

fn good(settings: &insta::Settings) {
    let _guard = settings.bind_to_scope();
}

fn main() {}
