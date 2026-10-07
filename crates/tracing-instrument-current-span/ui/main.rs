use tracing::{Instrument as _, Span};

async fn work() {}

fn main() {
    let _ = work().instrument(Span::current());
    let _ = work().in_current_span();
}
