use tracing::Span;

fn main() {
    drop(Span::none().enter());
}
