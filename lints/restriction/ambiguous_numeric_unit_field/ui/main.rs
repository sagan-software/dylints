struct DownloadLimits {
    maximum_size: u64,
    payload_length: usize,
    file_offset: i64,
    alias_size: RawSize,
    maximum_size_bytes: u64,
    byte_offset: u64,
    item_length: usize,
    /// Maximum accepted document size in bytes.
    documented_size: u64,
    #[doc = "Length in characters."]
    documented_length: usize,
    retries: u32,
    semantic_size: ByteCount,
    floating_size: f64,
    size_hint: u64,
}

type RawSize = u64;

struct ByteCount(u64);

fn main() {}
