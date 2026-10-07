/// Missing an examples section.
fn private_without_example() {}

/// # Examples
///
/// ```
/// private_with_example();
/// ```
fn private_with_example() {}

struct PrivateService;

impl PrivateService {
    /// Missing an examples section.
    fn private_method(&self) {}
}

fn main() {
    let service = PrivateService;
    service.private_method();
    private_without_example();
    private_with_example();
}
