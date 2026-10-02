fn manual(option: &mut Option<i32>) -> Option<i32> {
    // Trigger when both calls use the same local `Option`.
    if option.as_ref().is_some_and(|value| *value > 0) {
        option.take()
    } else {
        None
    }
}

struct Holder {
    slot: Option<i32>,
    pair: (Option<i32>, Option<i32>),
}

impl Holder {
    fn field(&mut self) -> Option<i32> {
        // Trigger for the same field of the same binding.
        if self.slot.as_ref().is_some_and(|value| *value > 0) {
            self.slot.take()
        } else {
            None
        }
    }

    fn other_field(&mut self) -> Option<i32> {
        // Keep quiet when the condition and the take name different fields.
        if self.pair.0.as_ref().is_some_and(|value| *value > 0) {
            self.pair.1.take()
        } else {
            None
        }
    }
}

fn dereferenced(option: &mut Option<i32>) -> Option<i32> {
    // Trigger for the same explicit dereference.
    if (*option).as_ref().is_some_and(|value| *value > 0) {
        (*option).take()
    } else {
        None
    }
}

fn mixed_dereference(option: &mut Option<i32>) -> Option<i32> {
    // Keep quiet when only one receiver is written as a dereference.
    if (*option).as_ref().is_some_and(|value| *value > 0) {
        option.take()
    } else {
        None
    }
}

fn different_receiver(first: &mut Option<i32>, second: &mut Option<i32>) -> Option<i32> {
    // Keep quiet when the condition and the take name different options.
    if first.as_ref().is_some_and(|value| *value > 0) {
        second.take()
    } else {
        None
    }
}

fn other_default(option: &mut Option<i32>) -> Option<i32> {
    // Keep quiet when the `else` branch is not `None`.
    if option.as_ref().is_some_and(|value| *value > 0) {
        option.take()
    } else {
        Some(0)
    }
}

fn else_block(option: &mut Option<i32>) -> Option<i32> {
    // Keep quiet when the `else` branch has more than one expression.
    if option.as_ref().is_some_and(|value| *value > 0) {
        option.take()
    } else {
        let fallback = None;
        fallback
    }
}

fn else_if(option: &mut Option<i32>, fallback: bool) -> Option<i32> {
    // Keep quiet when the `else` branch is another conditional.
    if option.as_ref().is_some_and(|value| *value > 0) {
        option.take()
    } else if fallback {
        None
    } else {
        Some(0)
    }
}

fn cloned(option: &mut Option<i32>) -> Option<i32> {
    // Keep quiet when the branch calls a trait method instead of `take`.
    if option.as_ref().is_some_and(|value| *value > 0) {
        option.clone()
    } else {
        None
    }
}

fn mutable_borrow(option: &mut Option<i32>) -> Option<i32> {
    // Keep quiet for a condition that does not borrow through `as_ref`.
    if option.as_mut().is_some_and(|value| *value > 0) {
        option.take()
    } else {
        None
    }
}

fn replaced(option: &mut Option<i32>) -> Option<i32> {
    // Keep quiet when the branch is not a method call.
    if option.as_ref().is_some_and(|value| *value > 0) {
        std::mem::take(option)
    } else {
        None
    }
}

fn is_some(option: &mut Option<i32>) -> Option<i32> {
    // Keep quiet for another condition.
    if option.is_some() {
        option.take()
    } else {
        None
    }
}

struct Slot(Option<i32>);

impl Slot {
    fn as_ref(&self) -> Option<&i32> {
        self.0.as_ref()
    }

    fn take(&mut self) -> Option<i32> {
        self.0.take()
    }
}

fn lookalike(slot: &mut Slot) -> Option<i32> {
    // Keep quiet for methods of a local type.
    if slot.as_ref().is_some_and(|value| *value > 0) {
        slot.take()
    } else {
        None
    }
}

fn main() {
    let mut first = Some(1);
    let mut second = Some(2);
    let mut holder = Holder {
        slot: Some(1),
        pair: (Some(1), Some(2)),
    };
    let _ = manual(&mut first);
    let _ = holder.field();
    let _ = holder.other_field();
    let _ = dereferenced(&mut first);
    let _ = mixed_dereference(&mut first);
    let _ = different_receiver(&mut first, &mut second);
    let _ = other_default(&mut first);
    let _ = else_block(&mut first);
    let _ = else_if(&mut first, true);
    let _ = cloned(&mut first);
    let _ = mutable_borrow(&mut first);
    let _ = replaced(&mut first);
    let _ = is_some(&mut first);
    let _ = lookalike(&mut Slot(Some(1)));
}
