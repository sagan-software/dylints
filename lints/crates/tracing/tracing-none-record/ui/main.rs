use tracing::Span;

fn main() {
    Span::none().record("field", 1);
}
