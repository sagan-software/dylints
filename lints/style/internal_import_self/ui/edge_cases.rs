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

use self::local::QualifiedFromSelf;
use crate::local::QualifiedFromCrate;
use crate::sibling::SiblingItem as RootSiblingItem;
use local::AliasItem as RenamedItem;
use local::Item;
use local::{GroupedOne, GroupedTwo};
use std::{fmt, io};

fn main() {
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
