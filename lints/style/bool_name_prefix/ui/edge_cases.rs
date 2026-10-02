#![allow(dead_code, non_upper_case_globals, unused_variables)]

type Flag = bool;

const READY_FLAG: bool = true;
const IS_READY: bool = true;

struct Flags {
    enabled: bool,
    is_enabled: bool,
    has_access: Flag,
}

trait Checks {
    fn available(&self) -> bool;
    fn is_available(&self) -> bool;
}

fn active() -> bool {
    true
}

fn accepts_flag(visible: bool, is_visible: bool) {
    let ready = visible && is_visible;
    let is_ready = ready;
    let _ = is_ready;
}

trait AliasChecks {
    const ALIAS_READY: Flag;

    fn alias_ready(&self, alias_visible: Flag, _: bool) -> Flag;

    fn provided_alias(&self) -> Flag {
        true
    }
}

struct Aliased;

impl Aliased {
    const ALIAS_DONE: Flag = true;

    fn alias_done(&self) -> Flag {
        true
    }

    fn ignored_parameter(&self, _: bool) {}
}

macro_rules! generated_flag {
    () => {
        fn generated() -> bool {
            true
        }
    };
}

generated_flag!();

fn main() {}
