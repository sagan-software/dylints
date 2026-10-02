macro_rules! info {
    ($($tokens:tt)*) => {};
}

macro_rules! warn {
    ($($tokens:tt)*) => {};
}

macro_rules! debug {
    ($($tokens:tt)*) => {};
}

macro_rules! error {
    ($($tokens:tt)*) => {};
}

fn main() {
    let user_id = 42;

    info!("user {} logged in", user_id);
    warn!("request failed for {user_id}");
    debug!(user_id = user_id, "user logged in");
    error!("static startup message");
}

fn writes_snapshot() -> std::io::Result<()> {
    use std::io::Write as _;

    writeln!(
        std::io::stdout().lock(),
        "stage={} step={} living={}/{} food={} well={:.2} observations={} global_state={}",
        "training",
        7,
        3,
        4,
        5,
        6.0,
        8,
        9,
    )?;

    Ok(())
}
