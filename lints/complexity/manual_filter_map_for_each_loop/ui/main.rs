fn parse(value: &str) -> Option<i32> {
    value.parse().ok()
}

fn consume(_value: i32) {}

fn conditional(values: &[&str]) {
    for value in values {
        if let Some(mapped) = parse(value) {
            consume(mapped);
        }
    }
}

fn result_conversion(values: &[&str]) {
    for value in values {
        if let Some(mapped) = value.parse::<i32>().ok() {
            consume(mapped);
        }
    }
}

fn main() {
    conditional(&["1"]);
    result_conversion(&["1"]);
}
