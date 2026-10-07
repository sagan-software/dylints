#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

// The ordinary example build compiles none of the test code. The `--test`
// build compiles it, and the lint skips test harness builds.

fn main() {}

#[cfg(test)]
mod tests {
    use bevy::ecs::{resource::Resource, world::World};

    #[derive(Resource)]
    struct Score(u32);

    #[test]
    fn reads_the_score() {
        let mut world = World::new();
        world.insert_resource(Score(1));
        assert_eq!(world.resource::<Score>().0, 1);
    }
}
