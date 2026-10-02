use std::{io, ops::ControlFlow};

fn write_value(value: &i32) -> io::Result<()> {
    if *value < 0 {
        return Err(io::Error::other("negative"));
    }
    Ok(())
}

fn manual(values: &[i32]) -> io::Result<()> {
    for value in values {
        write_value(value)?;
    }
    Ok(())
}

fn multiple_actions(values: &[i32]) -> io::Result<()> {
    for value in values {
        write_value(value)?;
        println!("{value}");
    }
    Ok(())
}

fn optional_action(value: &i32) -> Option<()> {
    (*value >= 0).then_some(())
}

fn manual_option(values: &[i32]) -> Option<()> {
    for value in values {
        optional_action(value)?;
    }
    Some(())
}

fn control_action(value: &i32) -> ControlFlow<&'static str> {
    if *value < 0 {
        return ControlFlow::Break("negative");
    }
    ControlFlow::Continue(())
}

fn manual_control_flow(values: &[i32]) -> ControlFlow<&'static str> {
    for value in values {
        control_action(value)?;
    }
    ControlFlow::Continue(())
}

fn action_after_loop(values: &[i32]) -> io::Result<()> {
    for value in values {
        write_value(value)?;
    }
    println!("finished");
    Ok(())
}

fn main() {
    let _ = manual(&[1, 2]);
    let _ = multiple_actions(&[1, 2]);
    let _ = manual_option(&[1, 2]);
    let _ = manual_control_flow(&[1, 2]);
    let _ = action_after_loop(&[1, 2]);
}
