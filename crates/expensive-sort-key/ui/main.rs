use std::string::String as Label;

struct Record {
    name: Label,
    rank: u32,
}

fn sort_lowercase(records: &mut [Record]) {
    records.sort_by_key(|record| record.name.to_lowercase());
}

fn sort_uppercase(records: &mut [Record]) {
    records.sort_by_key(|record| record.name.to_uppercase());
}

fn sort_ascii_lowercase(records: &mut [Record]) {
    records.sort_by_key(|record| record.name.to_ascii_lowercase());
}

fn sort_ascii_uppercase(records: &mut [Record]) {
    records.sort_by_key(|record| record.name.to_ascii_uppercase());
}

struct BorrowedRecord {
    name: &'static str,
}

fn sort_borrowed_str(records: &mut [BorrowedRecord]) {
    records.sort_by_key(|record| record.name.to_lowercase());
}

fn record_key(record: &Record) -> String {
    record.name.to_lowercase()
}

fn sort_function_item(records: &mut [Record]) {
    records.sort_by_key(record_key);
}

// Keep the explicit closure block to exercise the distinct HIR block shape.
#[rustfmt::skip]
fn sort_empty_block(records: &mut [Record]) {
    records.sort_by_key(|record| { record.name.to_lowercase() });
}

fn sort_unit_block(records: &mut [Record]) {
    records.sort_by_key(|record| {});
}

fn sort_destructured(records: &mut [Record]) {
    records.sort_by_key(|Record { name, .. }| name.to_lowercase());
}

struct IndexedRecord {
    names: [String; 1],
}

fn sort_indexed_field(records: &mut [IndexedRecord]) {
    records.sort_by_key(|record| record.names[0].to_lowercase());
}

fn sort_owned_clone(names: &mut [Label]) {
    names.sort_by_key(|name| (*name).clone());
}

fn sort_empty_array() {
    let mut names: [Label; 0] = [];
    names.sort_by_key(|name| name.to_lowercase());
}

fn sort_singleton_array() {
    let mut names = [Label::from("solo")];
    names.sort_by_key(|name| name.to_lowercase());
}

fn sort_singleton_array_reference(names: &mut [Label; 1]) {
    names.sort_by_key(|name| name.to_lowercase());
}

fn sort_two_element_array(names: &mut [Label; 2]) {
    names.sort_by_key(|name| name.to_lowercase());
}

fn sort_three_element_array(names: &mut [Label; 3]) {
    names.sort_by_key(|name| name.to_lowercase());
}

fn sort_generic_array<const N: usize>(names: &mut [Label; N]) {
    names.sort_by_key(|name| name.to_lowercase());
}

fn sort_cheap(records: &mut [Record]) {
    records.sort_by_key(|record| record.rank * 2);
}

fn sort_cached(records: &mut [Record]) {
    records.sort_by_cached_key(|record| record.name.to_lowercase());
}

fn lowercase(name: &str) -> String {
    name.to_lowercase()
}

fn sort_helper(records: &mut [Record]) {
    records.sort_by_key(|record| lowercase(&record.name));
}

struct LocalName(String);

impl LocalName {
    fn to_lowercase(&self) -> String {
        self.0.clone()
    }
}

fn sort_lookalike(names: &mut [LocalName]) {
    names.sort_by_key(|name| name.to_lowercase());
}

struct NumericName;

impl NumericName {
    fn to_lowercase(&self) -> usize {
        0
    }
}

fn sort_non_string_lookalike(names: &mut [NumericName]) {
    names.sort_by_key(|name| name.to_lowercase());
}

struct CustomDeref(String);

impl std::ops::Deref for CustomDeref {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn sort_custom_deref(names: &mut [CustomDeref]) {
    names.sort_by_key(|name| name.to_lowercase());
}

fn sort_side_effecting(records: &mut [Record]) {
    let mut calls = 0;
    records.sort_by_key(|record| {
        calls += 1;
        record.name.to_lowercase()
    });
}

struct CustomSort(Vec<Record>);

impl CustomSort {
    fn sort_by_key<F, K>(&mut self, _key: F)
    where
        F: FnMut(&Record) -> K,
    {
        let _ = self.0.len();
    }
}

fn sort_lookalike_method(records: &mut CustomSort) {
    records.sort_by_key(|record| record.name.to_lowercase());
}

macro_rules! sort_through_macro {
    ($records:expr) => {
        $records.sort_by_key(|record| record.name.to_lowercase())
    };
}

fn sort_macro_expansion(records: &mut [Record]) {
    sort_through_macro!(records);
}

fn main() {}
