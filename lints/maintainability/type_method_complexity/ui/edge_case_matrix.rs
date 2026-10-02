#![allow(dead_code)]

// Each impl is one independent aggregate case below the configured limit.

// Case 001: four methods with 1 decision each.
struct Case001;
impl Case001 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 002: four methods with 2 decisions each.
struct Case002;
impl Case002 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 003: four methods with 3 decisions each.
struct Case003;
impl Case003 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 004: four methods with 1 decision each.
struct Case004;
impl Case004 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 005: four methods with 2 decisions each.
struct Case005;
impl Case005 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 006: four methods with 3 decisions each.
struct Case006;
impl Case006 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 007: four methods with 1 decision each.
struct Case007;
impl Case007 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 008: four methods with 2 decisions each.
struct Case008;
impl Case008 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 009: four methods with 3 decisions each.
struct Case009;
impl Case009 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 010: four methods with 1 decision each.
struct Case010;
impl Case010 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 011: four methods with 2 decisions each.
struct Case011;
impl Case011 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 012: four methods with 3 decisions each.
struct Case012;
impl Case012 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 013: four methods with 1 decision each.
struct Case013;
impl Case013 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 014: four methods with 2 decisions each.
struct Case014;
impl Case014 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 015: four methods with 3 decisions each.
struct Case015;
impl Case015 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 016: four methods with 1 decision each.
struct Case016;
impl Case016 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 017: four methods with 2 decisions each.
struct Case017;
impl Case017 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 018: four methods with 3 decisions each.
struct Case018;
impl Case018 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 019: four methods with 1 decision each.
struct Case019;
impl Case019 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 020: four methods with 2 decisions each.
struct Case020;
impl Case020 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 021: four methods with 3 decisions each.
struct Case021;
impl Case021 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 022: four methods with 1 decision each.
struct Case022;
impl Case022 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 023: four methods with 2 decisions each.
struct Case023;
impl Case023 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 024: four methods with 3 decisions each.
struct Case024;
impl Case024 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 025: four methods with 1 decision each.
struct Case025;
impl Case025 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 026: four methods with 2 decisions each.
struct Case026;
impl Case026 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 027: four methods with 3 decisions each.
struct Case027;
impl Case027 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 028: four methods with 1 decision each.
struct Case028;
impl Case028 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 029: four methods with 2 decisions each.
struct Case029;
impl Case029 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 030: four methods with 3 decisions each.
struct Case030;
impl Case030 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 031: four methods with 1 decision each.
struct Case031;
impl Case031 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 032: four methods with 2 decisions each.
struct Case032;
impl Case032 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 033: four methods with 3 decisions each.
struct Case033;
impl Case033 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 034: four methods with 1 decision each.
struct Case034;
impl Case034 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 035: four methods with 2 decisions each.
struct Case035;
impl Case035 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 036: four methods with 3 decisions each.
struct Case036;
impl Case036 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 037: four methods with 1 decision each.
struct Case037;
impl Case037 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 038: four methods with 2 decisions each.
struct Case038;
impl Case038 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 039: four methods with 3 decisions each.
struct Case039;
impl Case039 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 040: four methods with 1 decision each.
struct Case040;
impl Case040 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 041: four methods with 2 decisions each.
struct Case041;
impl Case041 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 042: four methods with 3 decisions each.
struct Case042;
impl Case042 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 043: four methods with 1 decision each.
struct Case043;
impl Case043 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 044: four methods with 2 decisions each.
struct Case044;
impl Case044 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 045: four methods with 3 decisions each.
struct Case045;
impl Case045 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 046: four methods with 1 decision each.
struct Case046;
impl Case046 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 047: four methods with 2 decisions each.
struct Case047;
impl Case047 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 048: four methods with 3 decisions each.
struct Case048;
impl Case048 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 049: four methods with 1 decision each.
struct Case049;
impl Case049 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 050: four methods with 2 decisions each.
struct Case050;
impl Case050 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 051: four methods with 3 decisions each.
struct Case051;
impl Case051 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 052: four methods with 1 decision each.
struct Case052;
impl Case052 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 053: four methods with 2 decisions each.
struct Case053;
impl Case053 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 054: four methods with 3 decisions each.
struct Case054;
impl Case054 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 055: four methods with 1 decision each.
struct Case055;
impl Case055 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 056: four methods with 2 decisions each.
struct Case056;
impl Case056 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 057: four methods with 3 decisions each.
struct Case057;
impl Case057 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 058: four methods with 1 decision each.
struct Case058;
impl Case058 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 059: four methods with 2 decisions each.
struct Case059;
impl Case059 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 060: four methods with 3 decisions each.
struct Case060;
impl Case060 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 061: four methods with 1 decision each.
struct Case061;
impl Case061 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 062: four methods with 2 decisions each.
struct Case062;
impl Case062 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 063: four methods with 3 decisions each.
struct Case063;
impl Case063 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 064: four methods with 1 decision each.
struct Case064;
impl Case064 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 065: four methods with 2 decisions each.
struct Case065;
impl Case065 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 066: four methods with 3 decisions each.
struct Case066;
impl Case066 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 067: four methods with 1 decision each.
struct Case067;
impl Case067 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 068: four methods with 2 decisions each.
struct Case068;
impl Case068 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 069: four methods with 3 decisions each.
struct Case069;
impl Case069 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 070: four methods with 1 decision each.
struct Case070;
impl Case070 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 071: four methods with 2 decisions each.
struct Case071;
impl Case071 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 072: four methods with 3 decisions each.
struct Case072;
impl Case072 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 073: four methods with 1 decision each.
struct Case073;
impl Case073 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 074: four methods with 2 decisions each.
struct Case074;
impl Case074 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 075: four methods with 3 decisions each.
struct Case075;
impl Case075 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 076: four methods with 1 decision each.
struct Case076;
impl Case076 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 077: four methods with 2 decisions each.
struct Case077;
impl Case077 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 078: four methods with 3 decisions each.
struct Case078;
impl Case078 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 079: four methods with 1 decision each.
struct Case079;
impl Case079 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 080: four methods with 2 decisions each.
struct Case080;
impl Case080 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 081: four methods with 3 decisions each.
struct Case081;
impl Case081 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 082: four methods with 1 decision each.
struct Case082;
impl Case082 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 083: four methods with 2 decisions each.
struct Case083;
impl Case083 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 084: four methods with 3 decisions each.
struct Case084;
impl Case084 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 085: four methods with 1 decision each.
struct Case085;
impl Case085 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 086: four methods with 2 decisions each.
struct Case086;
impl Case086 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 087: four methods with 3 decisions each.
struct Case087;
impl Case087 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

// Case 088: four methods with 1 decision each.
struct Case088;
impl Case088 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    }
}

// Case 089: four methods with 2 decisions each.
struct Case089;
impl Case089 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    }
}

// Case 090: four methods with 3 decisions each.
struct Case090;
impl Case090 {
    fn method_1(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_2(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_3(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
    fn method_4(&self, flags: [bool; 3]) -> usize {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        if flags[2] {
            hits += 1;
        }
        hits
    }
}

fn main() {}
