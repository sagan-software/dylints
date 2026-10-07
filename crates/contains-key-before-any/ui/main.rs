// run-rustfix
// rustfix-only-machine-applicable
use std::collections::HashMap;

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

fn hash_map_receiver(document: &HashMap<String, String>) -> bool {
    document.contains_key("address") || ["full_name"].iter().any(|key| document.contains_key(*key))
}

fn empty_array(document: &Document) -> bool {
    let keys: [&str; 0] = [];
    document.contains_key("address") || keys.iter().any(|key| document.contains_key(*key))
}

macro_rules! generated_gate {
    ($document:expr) => {
        $document.contains_key("address")
            || ["full_name"].iter().any(|key| $document.contains_key(*key))
    };
}

fn macro_generated(document: &Document) -> bool {
    generated_gate!(document)
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
    let _ = hash_map_receiver(&HashMap::new());
    let _ = empty_array(&document);
    let _ = macro_generated(&document);
}

fn plain_rhs(document: &Document) -> bool {
    document.contains_key("address") || false
}

fn plain_lhs(document: &Document) -> bool {
    computed_key().is_empty() || ["full_name"].iter().any(|key| document.contains_key(*key))
}

fn saved_iterator(document: &Document) -> bool {
    let mut keys = ["full_name"].iter();
    document.contains_key("address") || keys.any(|key| document.contains_key(*key))
}

fn saved_predicate(document: &Document) -> bool {
    let predicate = |key: &&str| document.contains_key(*key);
    document.contains_key("address") || ["full_name"].iter().any(predicate)
}

fn destructured_key(document: &Document) -> bool {
    document.contains_key("address") || ["full_name"].iter().any(|&key| document.contains_key(key))
}

fn literal_receiver() -> bool {
    Document.contains_key("address") || ["full_name"].iter().any(|key| Document.contains_key(*key))
}

fn computed_receiver() -> bool {
    fn document() -> Document {
        Document
    }
    document().contains_key("address")
        || ["full_name"]
            .iter()
            .any(|key| document().contains_key(*key))
}

// A different one-argument method cannot be folded into the key search.
fn different_method_name(document: &Document) -> bool {
    document.has_key("address") || ["full_name"].iter().any(|key| document.contains_key(*key))
}

impl Document {
    fn has_key(&self, _key: &str) -> bool {
        true
    }
}
