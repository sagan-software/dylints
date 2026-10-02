#![allow(dead_code)]

use std::borrow::Borrow as StdBorrow;

type NameRef<'a> = &'a str;

struct UserName(String);
struct ImplementedName(String);
struct BorrowedName<'a>(&'a str);
struct BorrowPolicy;

impl UserName {
    const KIND: &'static str = "user";

    fn borrow_str(&self) -> &str {
        &self.0
    }

    fn borrow_name(&self) -> NameRef<'_> {
        &self.0
    }

    fn as_str(&self) -> &str {
        &self.0
    }

    fn borrow_owned_name(&self) -> String {
        self.0.clone()
    }

    fn borrow_wrapper(&self) -> BorrowedName<'_> {
        BorrowedName(&self.0)
    }

    fn borrow_for_policy(&self, _policy: BorrowPolicy) -> &str {
        &self.0
    }

    fn borrow_mut_str(&mut self) -> &mut str {
        self.0.as_mut_str()
    }

    fn borrow_from_mut(&mut self) -> &str {
        &self.0
    }

    fn borrow_other(other: &Self) -> &str {
        &other.0
    }
}

impl ImplementedName {
    fn borrow_str(&self) -> &str {
        &self.0
    }
}

impl StdBorrow<str> for ImplementedName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

trait BorrowView {
    fn borrow_view(&self) -> &str;
}

impl BorrowView for UserName {
    fn borrow_view(&self) -> &str {
        &self.0
    }
}

fn main() {}
