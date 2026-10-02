// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code, unused_imports)]

mod local {
    pub struct AliasItem;
    pub struct GroupedOne;
    pub struct GroupedTwo;
    pub struct Item;
    pub struct QualifiedFromCrate;
    pub struct QualifiedFromSelf;
}

mod sibling {
    pub struct SiblingItem;
}

mod grouped {
    pub mod nested {
        pub struct Deep;
    }

    pub struct Shallow;
}

mod parent {
    pub mod child {
        pub struct AliasChild;
        pub struct ChildItem;
        pub struct GroupedChild;
        pub struct QualifiedChildSelf;
    }

    #[cfg(any())]
    use child::DisabledChild;

    use self::child::AliasChild as RenamedChild;
    use self::child::QualifiedChildSelf;
    use self::child::{ChildItem, GroupedChild};
    use crate::sibling::SiblingItem;

    pub fn values() {
        let _ = (
            ChildItem,
            GroupedChild,
            QualifiedChildSelf,
            RenamedChild,
            SiblingItem,
        );
    }
}

#[cfg(any())]
use local::DisabledItem;

use crate::local::QualifiedFromCrate;
use crate::sibling::SiblingItem as RootSiblingItem;
use self::local::QualifiedFromSelf;
use local::AliasItem as RenamedItem;
use local::Item;
use grouped::{Shallow, nested::Deep};
use local::{GroupedOne, GroupedTwo};
use std::{fmt, io};

fn main() {
    let _ = (Deep, Shallow);
    let _ = (
        fmt::Error,
        GroupedOne,
        GroupedTwo,
        io::ErrorKind::Other,
        Item,
        QualifiedFromCrate,
        QualifiedFromSelf,
        RenamedItem,
        RootSiblingItem,
    );
    parent::values();
}
