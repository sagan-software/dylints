use tracing::Span;

fn main() {
    let _ = Span::none().or_current();
}
