fn decisions(values: [bool; 5]) -> usize {
    let mut count = 0;
    if values[0] {
        count += 1;
    }
    if values[1] {
        count += 1;
    }
    if values[2] {
        count += 1;
    }
    if values[3] {
        count += 1;
    }
    if values[4] {
        count += 1;
    }
    count
}
fn missing_coverage() {}
fn main() {
    let _count = decisions([true; 5]);
    missing_coverage();
}
