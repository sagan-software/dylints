use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

type NumberList = Vec<u64>;
type WordCounts<'a> = HashMap<&'a str, u64>;

pub fn doubled(values: &[u64]) -> Vec<u64> {
    values.iter().map(|value| value * 2).collect()
}

pub fn keyed<'a>(values: &'a [(&'a str, u64)]) -> HashMap<&'a str, u64> {
    values.iter().copied().collect()
}

pub fn aliased_numbers(values: &[u64]) -> NumberList {
    values.iter().copied().collect()
}

pub fn aliased_map<'a>(values: &'a [(&'a str, u64)]) -> WordCounts<'a> {
    values.iter().copied().collect()
}

pub fn qualified_set(values: &[u64]) -> std::collections::HashSet<u64> {
    values.iter().copied().collect()
}

pub fn boxed(values: &[u64]) -> Box<[u64]> {
    values.iter().copied().collect()
}

struct Names {
    values: Vec<String>,
}

impl Names {
    pub fn unique(&self) -> BTreeSet<&str> {
        self.values.iter().map(String::as_str).collect()
    }
}

fn private_unique(names: &Names) -> BTreeSet<&str> {
    names.values.iter().map(String::as_str).collect()
}

fn local_binding(values: &[u64]) -> Vec<u64> {
    let collected = values.iter().copied().collect();
    collected
}

fn explicit_inside_block(values: &[u64]) -> Vec<u64> {
    let mut out = values.iter().copied().collect::<Vec<_>>();
    out.push(1);
    out
}

fn fallible(values: &[u64]) -> Result<Vec<u64>, &'static str> {
    Ok(values.iter().copied().collect())
}

fn array_return(values: &[u64]) -> [u64; 2] {
    [values[0], values[1]]
}

fn string_return(values: &[&str]) -> String {
    values.join(",")
}

fn already_lazy(values: &[u64]) -> impl Iterator<Item = u64> + '_ {
    values.iter().copied()
}

pub fn btree_map<'a>(values: &'a [(&'a str, u64)]) -> BTreeMap<&'a str, u64> {
    values.iter().copied().collect()
}

pub fn hash_set(values: &[u64]) -> HashSet<u64> {
    values.iter().copied().collect()
}

struct CustomCollector;

impl CustomCollector {
    fn collect(self) -> Vec<u64> {
        vec![1, 2, 3]
    }
}

fn custom_collect_method() -> Vec<u64> {
    CustomCollector.collect()
}

fn main() {}
