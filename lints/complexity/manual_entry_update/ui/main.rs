use std::collections::{HashMap, hash_map::Entry};

fn manual(map: &mut HashMap<String, i32>, key: String) {
    // Trigger for one value update and one vacant insertion.
    match map.entry(key) {
        Entry::Occupied(mut entry) => *entry.get_mut() += 1,
        Entry::Vacant(entry) => {
            entry.insert(1);
        }
    }
}

fn occupied_only(map: &mut HashMap<String, i32>, key: String) {
    if let Entry::Occupied(mut entry) = map.entry(key) {
        *entry.get_mut() += 1;
    }
}

fn main() {
    let mut map = HashMap::new();
    manual(&mut map, String::from("a"));
    occupied_only(&mut map, String::from("a"));
}
