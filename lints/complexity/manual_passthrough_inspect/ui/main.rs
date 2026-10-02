fn operation(ok: bool) -> Result<i32, String> {
    ok.then_some(1).ok_or_else(|| String::from("error"))
}

fn inspect_error(ok: bool) -> Result<i32, String> {
    let result = operation(ok);
    if let Err(error) = &result {
        println!("{error}");
    }
    result
}

fn replace_error(ok: bool) -> Result<i32, String> {
    let result = operation(ok);
    if let Err(error) = &result {
        return Err(format!("wrapped: {error}"));
    }
    result
}

fn main() {
    let _ = inspect_error(false);
    let _ = replace_error(false);
}
