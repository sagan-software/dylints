#![allow(dead_code, missing_docs, unknown_lints)]

// The ordinary example build compiles none of the test code. The `--test`
// build compiles it, and the lint skips test harness builds.

fn main() {}

#[cfg(test)]
mod tests {
    use bevy::ecs::{component::Component, resource::Resource};

    #[derive(Component)]
    struct Marker;

    #[derive(Resource)]
    struct Fixture(u32);

    #[test]
    fn builds_fixtures() {
        let _ = (Marker, Fixture(1));
    }
}
