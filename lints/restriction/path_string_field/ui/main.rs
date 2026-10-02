struct PathBuf;

struct CacheConfig<'a> {
    cache_dir: String,
    manifest_path: &'a str,
    display_name: String,
    output_path: PathBuf,
}

fn main() {}
