use tracing::Span;

fn main() {
    let _ = Span::current().or_current();
}
