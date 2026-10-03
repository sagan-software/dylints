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

struct ListShape {
    value: u8,
}

impl Debug for ListShape {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_list().entry(&self.value).finish()
    }
}

struct BorrowedField {
    value: &'static u8,
}

impl Debug for BorrowedField {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("BorrowedField")
            .field("value", self.value)
            .finish()
    }
}

struct OtherReceiver {
    value: u8,
}

impl Debug for OtherReceiver {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("OtherReceiver")
            .field("value", &(Self { value: 9 }).value)
            .finish()
    }
}

/// Verify that the custom builder shapes preserve their exact output.
pub(crate) fn main() {
    // Compare custom builders and field expressions with their exact emitted text.
    let named = Named { value: 7 };
    let tuple = Tuple(7);
    let list = ListShape { value: 7 };
    let borrowed = BorrowedField { value: &7 };
    // The alternate receiver must print its own value rather than this stored value.
    let other = OtherReceiver { value: 7 };
    // Preserve the custom text when builders or field expressions differ from derives.
    assert_eq!(
        (
            format!("{named:?}"),
            format!("{tuple:?}"),
            format!("{list:?}"),
            format!("{borrowed:?}"),
            format!("{other:?}")
        ),
        (
            "Named(7)".to_owned(),
            "Tuple { 0: 7 }".to_owned(),
            "[7]".to_owned(),
            "BorrowedField { value: 7 }".to_owned(),
            "OtherReceiver { value: 9 }".to_owned()
        )
    );
}
