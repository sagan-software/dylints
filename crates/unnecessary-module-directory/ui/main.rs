#[path = "auxiliary/flat.rs"]
mod flat;
#[path = "auxiliary/multiple_files/mod.rs"]
mod multiple_files;
#[path = "auxiliary/single_file/mod.rs"]
mod single_file;

fn main() {
    assert_eq!(flat::value(), 1);
    assert_eq!(multiple_files::value(), 2);
    assert_eq!(single_file::value(), 3);
}
