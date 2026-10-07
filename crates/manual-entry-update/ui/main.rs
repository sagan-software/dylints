use std::collections::{BTreeMap, HashMap, btree_map, hash_map::Entry};

fn manual(map: &mut HashMap<String, i32>, key: String) {
    // Trigger for one value update and one vacant insertion.
    match map.entry(key) {
        Entry::Occupied(mut entry) => *entry.get_mut() += 1,
        Entry::Vacant(entry) => {
            entry.insert(1);
        }
    }
}

fn btree_reversed(map: &mut BTreeMap<String, Vec<i32>>, key: String, value: i32) {
    // Trigger for a B-tree entry, reversed arms, and a multi-statement update.
    match map.entry(key) {
        btree_map::Entry::Vacant(slot) => {
            slot.insert(vec![value]);
        }
        btree_map::Entry::Occupied(slot) => {
            let values = slot.into_mut();
            values.push(value);
        }
    }
}

fn closure_update(map: &mut HashMap<String, i32>, key: String) {
    // Trigger when the update uses the entry inside a closure.
    match map.entry(key) {
        Entry::Occupied(mut entry) => {
            let mut bump = || *entry.get_mut() += 1;
            bump();
        }
        Entry::Vacant(entry) => {
            entry.insert(1);
        }
    }
}

fn occupied_only(map: &mut HashMap<String, i32>, key: String) {
    // Keep quiet for an `if let` without a vacant branch.
    if let Entry::Occupied(mut entry) = map.entry(key) {
        *entry.get_mut() += 1;
    }
}

fn uses_key(map: &mut HashMap<String, usize>, key: String) {
    // Keep quiet when the vacant arm reads the entry before inserting.
    match map.entry(key) {
        Entry::Occupied(mut entry) => *entry.get_mut() += 1,
        Entry::Vacant(entry) => {
            let length = entry.key().len();
            entry.insert(length);
        }
    }
}

fn removes(map: &mut HashMap<String, i32>, key: String) {
    // Keep quiet when the occupied arm removes instead of updating.
    match map.entry(key) {
        Entry::Occupied(entry) => {
            entry.remove();
        }
        Entry::Vacant(entry) => {
            entry.insert(1);
        }
    }
}

fn reads_entry_twice(map: &mut HashMap<String, i32>, key: String) {
    // Keep quiet when the occupied arm uses the entry for more than the update.
    match map.entry(key) {
        Entry::Occupied(mut entry) => {
            let previous = *entry.get();
            *entry.get_mut() = previous + 1;
        }
        Entry::Vacant(entry) => {
            entry.insert(1);
        }
    }
}

fn returns_value(map: &mut HashMap<String, i32>, key: String) -> &mut i32 {
    // Keep quiet when the match produces the value reference.
    match map.entry(key) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(0),
    }
}

fn early_return(map: &mut HashMap<String, i32>, key: String, limit: i32) {
    // Keep quiet when the update can return from the function.
    match map.entry(key) {
        Entry::Occupied(mut entry) => {
            if *entry.get_mut() > limit {
                return;
            }
        }
        Entry::Vacant(entry) => {
            entry.insert(1);
        }
    }
}

fn fallible_insert(map: &mut HashMap<String, i32>, key: String, raw: &str) -> Option<()> {
    // Keep quiet when the inserted value uses `?`.
    match map.entry(key) {
        Entry::Occupied(mut entry) => *entry.get_mut() += 1,
        Entry::Vacant(entry) => {
            entry.insert(raw.parse().ok()?);
        }
    }
    Some(())
}

fn guarded(map: &mut HashMap<String, i32>, key: String, enabled: bool) {
    // Keep quiet when an arm has a guard.
    match map.entry(key) {
        Entry::Occupied(mut entry) if enabled => *entry.get_mut() += 1,
        Entry::Occupied(_) => {}
        Entry::Vacant(entry) => {
            entry.insert(1);
        }
    }
}

fn wildcard(map: &mut HashMap<String, i32>, key: String) {
    // Keep quiet when one arm is a wildcard.
    match map.entry(key) {
        Entry::Occupied(mut entry) => *entry.get_mut() += 1,
        _ => {}
    }
}

fn reference_binding(map: &mut HashMap<String, i32>, key: String) {
    // Keep quiet when an arm binds the entry by reference.
    match map.entry(key) {
        Entry::Occupied(ref mut entry) => *entry.get_mut() += 1,
        Entry::Vacant(entry) => {
            entry.insert(1);
        }
    }
}

fn vacant_key(map: &mut HashMap<String, i32>, key: String) {
    // Keep quiet when the vacant arm calls another method.
    match map.entry(key) {
        Entry::Occupied(mut entry) => *entry.get_mut() += 1,
        Entry::Vacant(entry) => {
            entry.into_key();
        }
    }
}

fn vacant_other(map: &mut HashMap<String, i32>, key: String, other: &mut Vec<i32>) {
    // Keep quiet when the vacant arm inserts into something other than the entry.
    match map.entry(key) {
        Entry::Occupied(mut entry) => *entry.get_mut() += 1,
        Entry::Vacant(_entry) => {
            other.push(1);
        }
    }
}

fn vacant_function(map: &mut HashMap<String, i32>, key: String) {
    // Keep quiet when the vacant arm is not a method call.
    match map.entry(key) {
        Entry::Occupied(mut entry) => *entry.get_mut() += 1,
        Entry::Vacant(entry) => drop(entry),
    }
}

fn sink(_value: i32) {}

enum Local {
    Occupied(i32),
    Vacant(i32),
}

fn lookalike(value: Local) {
    // Keep quiet for a local enum with the same variant names.
    match value {
        Local::Occupied(inner) => sink(inner),
        Local::Vacant(inner) => sink(inner),
    }
}

fn number(value: i32) {
    // Keep quiet for a match on a primitive.
    match value {
        0 => sink(0),
        _ => sink(1),
    }
}

fn option_entry(value: Option<i32>) {
    // Keep quiet for another standard enum.
    match value {
        Some(inner) => sink(inner),
        None => {}
    }
}

fn main() {
    let mut map = HashMap::new();
    manual(&mut map, String::from("a"));
    btree_reversed(&mut BTreeMap::new(), String::from("a"), 1);
    closure_update(&mut map, String::from("a"));
    occupied_only(&mut map, String::from("a"));
    uses_key(&mut HashMap::new(), String::from("a"));
    removes(&mut map, String::from("a"));
    reads_entry_twice(&mut map, String::from("a"));
    let _ = returns_value(&mut map, String::from("a"));
    early_return(&mut map, String::from("a"), 1);
    let _ = fallible_insert(&mut map, String::from("a"), "1");
    guarded(&mut map, String::from("a"), true);
    wildcard(&mut map, String::from("a"));
    reference_binding(&mut map, String::from("a"));
    vacant_key(&mut map, String::from("a"));
    vacant_other(&mut map, String::from("a"), &mut Vec::new());
    vacant_function(&mut map, String::from("a"));
    lookalike(Local::Occupied(1));
    lookalike(Local::Vacant(1));
    number(1);
    option_entry(Some(1));
}
