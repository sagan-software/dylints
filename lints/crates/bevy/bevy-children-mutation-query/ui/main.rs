#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::{
    change_detection::Mut,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    relationship::RelationshipTarget,
    system::Query,
};

fn bad(_: Query<&mut Children>) {}

fn detach_all(mut parents: Query<&mut Children>) {
    for mut children in &mut parents {
        children.collection_mut_risky().clear();
    }
}

fn reorder_then_escape(mut parents: Query<&mut Children>) {
    for mut children in &mut parents {
        children.swap(0, 1);
        take(children);
    }
}

fn reorder_through_reference(mut parents: Query<&mut Children>) {
    for mut children in &mut parents {
        let children: &mut Children = &mut children;
        children.sort_by(|left, right| left.cmp(right));
    }
}

fn take(_: Mut<Children>) {}

fn reorder(mut parents: Query<&mut Children>) {
    for mut children in &mut parents {
        if children.len() > 1 {
            children.swap(0, 1);
        }
    }
}

fn sort_in_closure(mut parents: Query<(Entity, &mut Children)>) {
    parents
        .iter_mut()
        .for_each(|(_, mut children)| children.sort_unstable_by_key(|child| *child));
}

fn good(_: Query<&ChildOf>) {}

trait Hook {
    fn run(&self, parents: Query<&mut Children>);
}

struct Detacher;

impl Hook for Detacher {
    fn run(&self, _: Query<&mut Children>) {}
}

fn main() {}
