struct PathBuf;

struct CacheConfig<'a> {
    cache_dir: String,
    manifest_path: &'a str,
    display_name: String,
    output_path: PathBuf,
    public_path: String,
    module_path: &'a str,
    url_path: String,
    route_path: String,
}

fn main() {}

struct OptionalCacheConfig {
    cache_dir: Option<String>,
    typed_cache_dir: Option<PathBuf>,
}
