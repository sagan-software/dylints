use std::fmt::{Debug, Formatter, Result};

struct Named {
    value: u8,
}

impl Debug for Named {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_tuple("Named").field(&self.value).finish()
    }
}

struct Tuple(u8);

impl Debug for Tuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Tuple").field("0", &self.0).finish()
    }
}

/// Verify that the custom builder shapes preserve their exact output.
pub(crate) fn main() {
    // Compare both custom builder shapes with their exact emitted text.
    let named = Named { value: 7 };
    let tuple = Tuple(7);
    assert_eq!(
        (format!("{named:?}"), format!("{tuple:?}")),
        ("Named(7)".to_owned(), "Tuple { 0: 7 }".to_owned())
    );
}
