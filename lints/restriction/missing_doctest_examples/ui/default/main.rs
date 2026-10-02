/// Missing an examples section.
pub fn undocumented_example() {}

/// # Examples
///
/// ```
/// documented_example();
/// ```
pub fn documented_example() {}

/// # Examples
///
/// ```text
/// This is not a Rust doctest.
/// ```
pub fn non_rust_example() {}

/// # Examples
///
/// ```rust
///
/// ```
pub fn empty_example() {}

/// # Example
///
/// ```
/// wrong_heading();
/// ```
pub fn wrong_heading() {}

struct PrivateService;

impl PrivateService {
    /// A public method on a private type is not externally reachable.
    pub fn unreachable_method(&self) {}
}

fn private_without_example() {}

pub struct Service;

impl Service {
    /// Missing an examples section.
    pub fn undocumented_method(&self) {}

    /// # Examples
    ///
    /// ```
    /// let service = Service;
    /// service.documented_method();
    /// ```
    pub fn documented_method(&self) {}
}

pub trait PublicTrait {
    /// Missing an examples section.
    fn trait_method(&self);

    /// # Examples
    ///
    /// ```
    /// let service = Service;
    /// service.provided_method();
    /// ```
    fn provided_method(&self) {
        // Keep one provided body so trait methods with bodies are covered.
    }
}

impl PublicTrait for Service {
    fn trait_method(&self) {}
}

/// Hidden from rustdoc, so it needs no example.
#[doc(hidden)]
pub fn hidden_helper() {}

/// Hidden module.
#[doc(hidden)]
pub mod hidden_module {
    /// Inside a hidden module.
    pub fn inside_hidden() {}
}

fn main() {
    let private_service = PrivateService;
    private_service.unreachable_method();
    private_without_example();
}
