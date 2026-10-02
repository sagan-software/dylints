use tokio::runtime::Runtime;

async fn bad() {
    let _ = Runtime::new();
}

fn good() {
    let _ = Runtime::new();
}

impl Holder {
    async fn bad_method(&self) {
        let _ = Runtime::new();
    }

    fn good_method(&self) {
        let _ = Runtime::new();
    }
}

/// A type with async and sync methods.
struct Holder;

fn main() {}
