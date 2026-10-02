fn manual(option: &mut Option<i32>) -> Option<i32> {
    if option.as_ref().is_some_and(|value| *value > 0) {
        option.take()
    } else {
        None
    }
}

fn different_receiver(first: &mut Option<i32>, second: &mut Option<i32>) -> Option<i32> {
    if first.as_ref().is_some_and(|value| *value > 0) {
        second.take()
    } else {
        None
    }
}

fn main() {
    let mut first = Some(1);
    let mut second = Some(2);
    let _ = manual(&mut first);
    let _ = different_receiver(&mut first, &mut second);
}
