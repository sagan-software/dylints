// compile-flags: --edition 2024
#![allow(dead_code, unreachable_code)]

use std::io;

fn write_value(value: &i32) -> io::Result<()> {
    if *value < 0 {
        return Err(io::Error::other("negative"));
    }
    Ok(())
}

fn question_mark_in_string(values: &[i32]) -> io::Result<()> {
    for value in values {
        write_value(value).map_err(|_| io::Error::other("why?"))?;
    }
    Ok(())
}

fn two_question_marks(values: &[i32]) -> io::Result<()> {
    for value in values {
        write_value(value).and_then(|()| write_value(value))?;
        write_value(value)?;
    }
    Ok(())
}

fn break_in_body(values: &[i32]) -> io::Result<()> {
    for value in values {
        if *value == 0 {
            break;
        }
        write_value(value)?;
    }
    Ok(())
}

fn conditional_break(values: &[i32]) -> io::Result<()> {
    for value in values {
        if *value == 0 {
            break;
        } else {
            write_value(value)?
        }
    }
    Ok(())
}

fn conditional_return(values: &[i32]) -> io::Result<()> {
    for value in values {
        if *value == 0 {
            return Ok(());
        } else {
            write_value(value)?
        }
    }
    Ok(())
}

fn closure_question_mark(values: &[i32]) -> io::Result<()> {
    for value in values {
        (|| -> io::Result<()> { write_value(value) })()?;
    }
    Ok(())
}

fn inner_block_tail(values: &[i32]) -> io::Result<()> {
    let result = {
        for value in values {
            write_value(value)?;
        }
        Ok(())
    };
    result
}

fn wrapped_block(values: &[i32]) -> io::Result<()> {
    {
        for value in values {
            write_value(value)?;
        }
        Ok(())
    }
}

fn closure_body(values: &[i32]) -> io::Result<()> {
    let run = || -> io::Result<()> {
        for value in values {
            write_value(value)?;
        }
        Ok(())
    };
    run()
}

async fn async_body(values: &[i32]) -> io::Result<()> {
    for value in values {
        write_value(value)?;
    }
    Ok(())
}

async fn ready(value: &i32) -> io::Result<()> {
    write_value(value)
}

async fn awaited_body(values: &[i32]) -> io::Result<()> {
    for value in values {
        ready(value).await?;
    }
    Ok(())
}

fn non_unit_tail(values: &[i32]) -> io::Result<usize> {
    for value in values {
        write_value(value)?;
    }
    Ok(values.len())
}

fn finish() {}

fn computed_unit_tail(values: &[i32]) -> io::Result<()> {
    for value in values {
        write_value(value)?;
    }
    Ok(finish())
}

fn qualified_tail(values: &[i32]) -> io::Result<()> {
    for value in values {
        write_value(value)?;
    }
    Result::Ok(())
}

macro_rules! success {
    () => {
        Ok(())
    };
}

fn macro_tail(values: &[i32]) -> io::Result<()> {
    for value in values {
        write_value(value)?;
    }
    success!()
}

#[allow(non_snake_case)]
fn local_constructor(values: &[i32]) -> io::Result<()> {
    fn Ok(value: ()) -> io::Result<()> {
        Result::Ok(value)
    }
    for value in values {
        write_value(value)?;
    }
    Ok(())
}

fn converted_error(values: &[i32]) -> Result<(), Box<dyn std::error::Error>> {
    for value in values {
        write_value(value)?;
    }
    Ok(())
}

fn main() {}

fn no_fallible_action(values: &[i32]) -> io::Result<()> {
    for value in values {
        println!("{value}");
    }
    Ok(())
}

fn discarded_result(values: &[i32]) -> io::Result<()> {
    for value in values {
        let _ = write_value(value);
    }
    Ok(())
}

fn error_tail(values: &[i32]) -> Result<(), ()> {
    for _value in values {
        Result::<(), ()>::Ok(())?;
    }
    Err(())
}
