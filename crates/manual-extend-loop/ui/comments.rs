// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

fn line_comment(output: &mut Vec<i32>, values: Vec<i32>) {
    for value in values {
        // Keep this explanation beside the per-item insertion.
        output.push(value);
    }
}

fn block_comment(output: &mut Vec<i32>, values: Vec<i32>) {
    for value in values {
        /* Keep this explanation beside the per-item insertion. */
        output.push(value);
    }
}

fn string_comment_markers(output: &mut Vec<&'static str>) {
    for value in ["// line marker", "/* block marker */"] {
        output.push(value);
    }
}

fn main() {}
