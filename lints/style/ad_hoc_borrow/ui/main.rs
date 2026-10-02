#![allow(dead_code)]

use std::borrow::Borrow as StdBorrow;

type NameRef<'a> = &'a str;

struct UserName(String);
struct BorrowedName<'a>(&'a str);
struct BorrowPolicy;

impl UserName {
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
}

impl StdBorrow<str> for UserName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

fn main() {}
