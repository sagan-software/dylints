fn operation(ok: bool) -> Result<i32, String> {
    ok.then_some(1).ok_or_else(|| String::from("error"))
}

fn lookup(ok: bool) -> Option<i32> {
    ok.then_some(1)
}

fn inspect_error(ok: bool) -> Result<i32, String> {
    let result = operation(ok);
    if let Err(error) = &result {
        println!("{error}");
    }
    result
}

fn inspect_success(ok: bool) -> Result<i32, String> {
    let result = operation(ok);
    if let Ok(value) = &result {
        println!("{value}");
    }
    result
}

fn keyword_like_name(ok: bool) -> Option<i32> {
    // A name that contains a keyword is not control flow.
    let returned = lookup(ok);
    if let Some(value) = &returned {
        println!("{value}")
    }
    returned
}

fn replace_error(ok: bool) -> Result<i32, String> {
    let result = operation(ok);
    if let Err(error) = &result {
        return Err(format!("wrapped: {error}"));
    }
    result
}

fn with_else(ok: bool) -> Option<i32> {
    // An `else` branch also observes the missing value.
    let found = lookup(ok);
    if let Some(value) = &found {
        println!("{value}")
    } else {
        println!("missing")
    }
    found
}

fn other_binding(ok: bool, other: Option<i32>) -> Option<i32> {
    // The observation reads a different binding.
    let found = lookup(ok);
    if let Some(value) = &other {
        println!("{value}");
    }
    found
}

fn borrowed_binding(ok: bool) -> Option<i32> {
    // The value is not borrowed in the observation.
    let found = lookup(ok);
    if let Some(value) = found {
        println!("{value}");
    }
    found
}

fn by_reference(found: &Option<i32>) -> &Option<i32> {
    // A borrowed binding cannot move into `inspect`.
    let found_ref = found;
    if let Some(value) = &found_ref {
        println!("{value}");
    }
    found_ref
}

fn two_actions(ok: bool) -> Option<i32> {
    let found = lookup(ok);
    if let Some(value) = &found {
        println!("{value}");
        println!("{value}");
    }
    found
}

fn early_exit(ok: bool) -> Option<i32> {
    // `?` exits the function from the observation.
    let found = lookup(ok);
    if let Some(value) = &found {
        println!("{}", value.checked_add(1)?);
    }
    found
}

fn other_tail(ok: bool) -> Option<i32> {
    let found = lookup(ok);
    if let Some(value) = &found {
        println!("{value}");
    }
    None
}

fn statement_before_binding(ok: bool) -> Option<i32> {
    println!("start");
    if let Some(value) = &lookup(ok) {
        println!("{value}");
    }
    lookup(ok)
}

fn not_if(ok: bool) -> Option<i32> {
    let found = lookup(ok);
    println!("{found:?}");
    found
}

fn boolean_test(ok: bool) -> Option<i32> {
    let found = lookup(ok);
    if found.is_some() {
        println!("found");
    }
    found
}

fn plain_value(value: i32) -> i32 {
    let doubled = value * 2;
    println!("{doubled}");
    doubled
}

macro_rules! generated {
    ($ok:expr) => {{
        let found = lookup($ok);
        if let Some(value) = &found {
            println!("{value}");
        }
        found
    }};
}

fn macro_block(ok: bool) -> Option<i32> {
    generated!(ok)
}

fn main() {
    let _ = inspect_error(false);
    let _ = replace_error(false);
}
