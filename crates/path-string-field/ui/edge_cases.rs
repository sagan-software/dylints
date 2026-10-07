#![allow(dead_code)]

use std::string::String as StdString;

struct PathBuf;
type PathText = std::string::String;
type PathStr = str;
struct PathTextWrapper(String);

struct Paths<'a> {
    file_path: PathText,
    output_dir: &'a PathStr,
    manifest_path: std::string::String,
    config_path: StdString,
    display_path_label: String,
    typed_path: PathBuf,
    wrapped_path: PathTextWrapper,
}

fn main() {}
