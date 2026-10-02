// run-rustfix
// rustfix-only-machine-applicable
use insta::Settings;

type Alias = Settings;

fn main() {
    let _ = insta::Settings::new();
    let _ = Settings::default();
    let _ = Alias::new();
    let _: Settings = Default::default();
    let _ = <Settings as Default>::default();
    let _ = insta::Settings::clone_current();
    let _: Vec<u8> = Default::default();
}
