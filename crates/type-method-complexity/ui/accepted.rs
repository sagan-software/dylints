#![allow(dead_code)]

struct Split;

macro_rules! generated_method {
    () => {
        fn generated(&self, values: [bool; 9]) {
            if values[0] {}
            if values[1] {}
            if values[2] {}
            if values[3] {}
            if values[4] {}
            if values[5] {}
            if values[6] {}
            if values[7] {}
            if values[8] {}
        }
    };
}

impl Split {
    fn first(&self, v: [bool; 8]) {
        if v[0] {}
        if v[1] {}
        if v[2] {}
        if v[3] {}
        if v[4] {}
        if v[5] {}
        if v[6] {}
        if v[7] {}
    }
    fn second(&self, v: [bool; 8]) {
        if v[0] {}
        if v[1] {}
        if v[2] {}
        if v[3] {}
        if v[4] {}
        if v[5] {}
        if v[6] {}
        if v[7] {}
    }
    fn third(&self, v: [bool; 8]) {
        if v[0] {}
        if v[1] {}
        if v[2] {}
        if v[3] {}
        if v[4] {}
        if v[5] {}
        if v[6] {}
        if v[7] {}
    }
}

impl Split {
    fn fourth(&self, v: [bool; 8]) {
        if v[0] {}
        if v[1] {}
        if v[2] {}
        if v[3] {}
        if v[4] {}
        if v[5] {}
        if v[6] {}
        if v[7] {}
    }
    fn fifth(&self, v: [bool; 8]) {
        if v[0] {}
        if v[1] {}
        if v[2] {}
        if v[3] {}
        if v[4] {}
        if v[5] {}
        if v[6] {}
        if v[7] {}
    }
    fn sixth(&self, v: [bool; 8]) {
        if v[0] {}
        if v[1] {}
        if v[2] {}
        if v[3] {}
        if v[4] {}
        if v[5] {}
        if v[6] {}
        if v[7] {}
    }
    generated_method!();
}

struct AtLimit;
impl AtLimit {
    fn one(&self, v: [bool; 9]) {
        if v[0] {}
        if v[1] {}
        if v[2] {}
        if v[3] {}
        if v[4] {}
        if v[5] {}
        if v[6] {}
        if v[7] {}
        if v[8] {}
    }
    fn two(&self, v: [bool; 9]) {
        if v[0] {}
        if v[1] {}
        if v[2] {}
        if v[3] {}
        if v[4] {}
        if v[5] {}
        if v[6] {}
        if v[7] {}
        if v[8] {}
    }
    fn three(&self, v: [bool; 9]) {
        if v[0] {}
        if v[1] {}
        if v[2] {}
        if v[3] {}
        if v[4] {}
        if v[5] {}
        if v[6] {}
        if v[7] {}
        if v[8] {}
    }
    fn four(&self, v: [bool; 9]) {
        if v[0] {}
        if v[1] {}
        if v[2] {}
        if v[3] {}
        if v[4] {}
        if v[5] {}
        if v[6] {}
        if v[7] {}
        if v[8] {}
    }
}

fn free_functions_do_not_count(v: [bool; 20]) {
    for flag in v {
        if flag {}
    }
}

trait WithDefault {
    fn provided(&self, v: [bool; 9]) {
        for flag in v {
            if flag {}
        }
    }
}

fn main() {}
