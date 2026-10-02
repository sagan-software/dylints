// compile-flags: --edition 2024
#![allow(dead_code, unused_variables, unused_assignments)]

fn consume<T>(_value: T) {}

fn contexts(values: &[String], other: &[String]) {
    // `let` initializers and assignments already name their boolean.
    let mut is_ready = values.iter().next().is_some() && other.iter().next().is_some();
    is_ready = values.iter().next().is_some() || other.iter().next().is_some();
    if values.iter().next().is_some() && other.iter().next().is_some() {}
    while values.iter().next().is_some() && other.iter().next().is_some() {
        break;
    }
    match values.len() {
        _ if values.iter().next().is_some() && other.iter().next().is_some() => (),
        _ => (),
    }
    if values.iter().any(|value| {
        let trimmed = value.trim();
        let lowered = trimmed.to_lowercase();
        lowered.starts_with('a') && lowered.ends_with('z')
    }) && !other.is_empty()
    {}
    if !values.is_empty() && !other.is_empty() {}
    if is_ready {}
}

fn main() {}

struct Chain;
impl Chain {
    fn step(&self) -> &Self {
        self
    }
    fn ready(&self) -> bool {
        true
    }
}

fn thresholds(chain: &Chain) {
    // Four calls score eight; five calls reach the standalone threshold of nine.
    if chain.step().step().step().ready() {}
    if !chain.step().step().step().step().ready() {}
    // Two six-point terms trigger; a six-point and a two-point term stay below nine.
    if chain.step().step().ready() && chain.step().step().ready() {}
    if chain.step().step().ready() && chain.ready() {}
    // One complex term triggers when the chain total reaches nine.
    if chain.step().step().ready() && !chain.step().ready() {}
    // Terms below six never count as complex, whatever the total.
    if chain.step().ready() && chain.step().ready() && chain.step().ready() {}
    if chain.step().step().ready() == chain.step().step().ready() {}
}

fn closures(chain: &Chain, values: &[i32]) {
    // A closure with one call or one comparison adds nothing.
    if values.iter().any(|value| *value > 0) && chain.step().ready() {}
    // Closure work above two adds to the term.
    if values
        .iter()
        .any(|value| *value > 0 && *value < 9 && *value != 4)
        && chain.step().ready()
    {}
    if values
        .iter()
        .any(|value| if *value > 1 { chain.ready() } else { false })
    {}
    if values.iter().any(|value| match *value {
        0 => chain.ready(),
        _ => false,
    }) {}
    if values.iter().any(|value| {
        loop {
            break *value > 0;
        }
    }) {}
    if values.iter().any(|value| {
        let mut result = *value;
        result = result + 1;
        result > 0
    }) {}
    // A nested closure counts as a branch.
    if values
        .iter()
        .any(|value| Some(*value).is_some_and(|inner| inner > 0))
    {}
    // Nested items are not closure work.
    if values.iter().any(|value| {
        fn unrelated() -> bool {
            let first = String::new();
            first.trim().to_lowercase().starts_with('a')
        }
        *value > 0
    }) {}
    if values.iter().any(|_| cfg!(debug_assertions)) {}
    // Long closures are capped at eight work units.
    if values.iter().any(|value| {
        let a = *value + 1;
        let b = a + 1;
        let c = b + 1;
        let d = c + 1;
        let e = d + 1;
        e > 0
    }) {}
    if accepts(|| {
        let a = chain.ready();
        let b = chain.step().ready();
        a && b
    }) {}
    // Closure work in an intermediate adapter contributes to the receiver chain.
    if [1, 2]
        .iter()
        .filter(|value| {
            let first = **value + 1;
            let second = first * 2;
            second > 2
        })
        .next()
        .is_some()
    {}
    if cfg!(debug_assertions) && chain.step().step().step().step().ready() {}
}

fn accepts(predicate: impl Fn() -> bool) -> bool {
    predicate()
}

struct Context {
    inner: Inner,
}

struct Inner {
    names: Vec<String>,
    is_enabled: bool,
}

impl Inner {
    fn lookup(&self) -> &Inner {
        self
    }
}

fn fields(context: &Context, other: &Inner) {
    // Fields on a named place are free.
    if context
        .inner
        .names
        .first()
        .is_some_and(|name| name.is_empty())
        && other.names.is_empty()
    {}
    if context.inner.is_enabled && other.is_enabled {}
    // Fields on a computed value add one plus the receiver chain.
    if context.inner.lookup().lookup().names.is_empty() && other.lookup().is_enabled {}
    if (context.inner.names.len() as u64) > 0 && &context.inner.names == &other.names {}
}

fn comparisons(value: &str, other: &str) {
    if value.trim().to_lowercase().trim().len() > 0 {}
    if value.trim().to_lowercase().len() > 0 && other.trim().to_lowercase().len() > 0 {}
    if value.trim().len() > 0 && other.trim().len() > 0 {}
    if value.len() > 0 && other.len() > 0 {}
}

fn pattern_conditions(values: &[i32], chain: &Chain) {
    if let Some(value) = values.first() {}
    while let Some(value) = values.first() {
        break;
    }
    let Some(value) = values.first() else {
        return;
    };
    if let Some(value) = values.first()
        && chain.step().step().ready()
    {}
    if let Some(value) = values.first()
        && chain.step().step().step().ready()
    {}
}

fn branches(chain: &Chain, flag: bool) {
    if { chain.step().ready() } && chain.step().step().ready() {}
    if (if flag { chain.ready() } else { false }) && chain.step().ready() {}
    if match flag {
        true if chain.ready() => chain.step().ready(),
        _ => false,
    } {}
    if !match flag {
        true => chain.ready(),
        false => chain.step().ready(),
    } {
        consume(());
    }
}

fn operand_matches(source: &str, flag: bool) -> Option<()> {
    // `?` adds nothing; the get, chars, and next calls score six plus one comparison.
    if source.get(1..)?.chars().next()? != '=' {}
    // A source `match` in a chain adds three.
    if match flag {
        true => source,
        false => "",
    }
    .trim()
    .len()
        > source.trim().trim().len()
    {}
    Some(())
}
