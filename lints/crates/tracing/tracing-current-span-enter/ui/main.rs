use tracing::Span;

fn main() {
    drop(Span::current().enter());
}
