struct Document;

impl Document {
    fn contains_key(&self, _key: &str) -> bool {
        false
    }
}

fn guarded_match(document: &Document) -> bool {
    // Reproduce the original `matches!` guard and its complete key list.
    matches!(
        document,
        _ if document.contains_key("address")
            || [
                "full_name",
                "date_of_birth",
                "country_of_birth",
                "city_of_birth",
                "client_identifier",
                "declares_no_client_identifier",
            ]
            .iter()
            .any(|key| document.contains_key(*key))
    )
}

fn direct_condition(document: &Document) -> bool {
    document.contains_key("address")
        || ["full_name", "date_of_birth"]
            .iter()
            .any(|key| document.contains_key(*key))
}

fn already_combined(document: &Document) -> bool {
    ["address", "full_name", "date_of_birth"]
        .iter()
        .any(|key| document.contains_key(*key))
}

fn different_receiver(first: &Document, second: &Document) -> bool {
    first.contains_key("address")
        || ["full_name", "date_of_birth"]
            .iter()
            .any(|key| second.contains_key(*key))
}

fn computed_key() -> &'static str {
    "full_name"
}

fn computed_array_key(document: &Document) -> bool {
    document.contains_key("address")
        || [computed_key(), "date_of_birth"]
            .iter()
            .any(|key| document.contains_key(*key))
}

fn extra_closure_condition(document: &Document) -> bool {
    document.contains_key("address")
        || ["full_name", "date_of_birth"]
            .iter()
            .any(|key| !key.is_empty() && document.contains_key(*key))
}

fn main() {
    // Exercise both warnings and every non-triggering contract boundary.
    let document = Document;
    let _ = guarded_match(&document);
    let _ = direct_condition(&document);
    let _ = already_combined(&document);
    let _ = different_receiver(&document, &document);
    let _ = computed_array_key(&document);
    let _ = extra_closure_condition(&document);
}
