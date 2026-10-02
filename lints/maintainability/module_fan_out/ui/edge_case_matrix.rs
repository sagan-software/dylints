#![allow(dead_code)]

mod target_1 {
    pub fn call() {}
}
mod target_2 {
    pub fn call() {}
}
mod target_3 {
    pub fn call() {}
}
mod target_4 {
    pub fn call() {}
}
mod target_5 {
    pub fn call() {}
}
mod target_6 {
    pub fn call() {}
}
mod target_7 {
    pub fn call() {}
}

// Case 001: 1 distinct dependencies below the limit.
mod source_001 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 002: 2 distinct dependencies below the limit.
mod source_002 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 003: 3 distinct dependencies below the limit.
mod source_003 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
    }
}

// Case 004: 4 distinct dependencies below the limit.
mod source_004 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 005: 5 distinct dependencies below the limit.
mod source_005 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 006: 6 distinct dependencies below the limit.
mod source_006 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
    }
}

// Case 007: 7 distinct dependencies below the limit.
mod source_007 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 008: 1 distinct dependencies below the limit.
mod source_008 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 009: 2 distinct dependencies below the limit.
mod source_009 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 010: 3 distinct dependencies below the limit.
mod source_010 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_1::call(); // A repeated edge still counts once.
    }
}

// Case 011: 4 distinct dependencies below the limit.
mod source_011 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 012: 5 distinct dependencies below the limit.
mod source_012 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 013: 6 distinct dependencies below the limit.
mod source_013 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
    }
}

// Case 014: 7 distinct dependencies below the limit.
mod source_014 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 015: 1 distinct dependencies below the limit.
mod source_015 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 016: 2 distinct dependencies below the limit.
mod source_016 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 017: 3 distinct dependencies below the limit.
mod source_017 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
    }
}

// Case 018: 4 distinct dependencies below the limit.
mod source_018 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 019: 5 distinct dependencies below the limit.
mod source_019 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 020: 6 distinct dependencies below the limit.
mod source_020 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_1::call(); // A repeated edge still counts once.
    }
}

// Case 021: 7 distinct dependencies below the limit.
mod source_021 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 022: 1 distinct dependencies below the limit.
mod source_022 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 023: 2 distinct dependencies below the limit.
mod source_023 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 024: 3 distinct dependencies below the limit.
mod source_024 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
    }
}

// Case 025: 4 distinct dependencies below the limit.
mod source_025 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 026: 5 distinct dependencies below the limit.
mod source_026 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 027: 6 distinct dependencies below the limit.
mod source_027 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
    }
}

// Case 028: 7 distinct dependencies below the limit.
mod source_028 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 029: 1 distinct dependencies below the limit.
mod source_029 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 030: 2 distinct dependencies below the limit.
mod source_030 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_1::call(); // A repeated edge still counts once.
    }
}

// Case 031: 3 distinct dependencies below the limit.
mod source_031 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
    }
}

// Case 032: 4 distinct dependencies below the limit.
mod source_032 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 033: 5 distinct dependencies below the limit.
mod source_033 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 034: 6 distinct dependencies below the limit.
mod source_034 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
    }
}

// Case 035: 7 distinct dependencies below the limit.
mod source_035 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 036: 1 distinct dependencies below the limit.
mod source_036 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 037: 2 distinct dependencies below the limit.
mod source_037 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 038: 3 distinct dependencies below the limit.
mod source_038 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
    }
}

// Case 039: 4 distinct dependencies below the limit.
mod source_039 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 040: 5 distinct dependencies below the limit.
mod source_040 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_1::call(); // A repeated edge still counts once.
    }
}

// Case 041: 6 distinct dependencies below the limit.
mod source_041 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
    }
}

// Case 042: 7 distinct dependencies below the limit.
mod source_042 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 043: 1 distinct dependencies below the limit.
mod source_043 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 044: 2 distinct dependencies below the limit.
mod source_044 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 045: 3 distinct dependencies below the limit.
mod source_045 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
    }
}

// Case 046: 4 distinct dependencies below the limit.
mod source_046 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 047: 5 distinct dependencies below the limit.
mod source_047 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 048: 6 distinct dependencies below the limit.
mod source_048 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
    }
}

// Case 049: 7 distinct dependencies below the limit.
mod source_049 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 050: 1 distinct dependencies below the limit.
mod source_050 {
    pub fn run() {
        crate::target_1::call();
        crate::target_1::call(); // A repeated edge still counts once.
    }
}

// Case 051: 2 distinct dependencies below the limit.
mod source_051 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 052: 3 distinct dependencies below the limit.
mod source_052 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
    }
}

// Case 053: 4 distinct dependencies below the limit.
mod source_053 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 054: 5 distinct dependencies below the limit.
mod source_054 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 055: 6 distinct dependencies below the limit.
mod source_055 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
    }
}

// Case 056: 7 distinct dependencies below the limit.
mod source_056 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 057: 1 distinct dependencies below the limit.
mod source_057 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 058: 2 distinct dependencies below the limit.
mod source_058 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 059: 3 distinct dependencies below the limit.
mod source_059 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
    }
}

// Case 060: 4 distinct dependencies below the limit.
mod source_060 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_1::call(); // A repeated edge still counts once.
    }
}

// Case 061: 5 distinct dependencies below the limit.
mod source_061 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 062: 6 distinct dependencies below the limit.
mod source_062 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
    }
}

// Case 063: 7 distinct dependencies below the limit.
mod source_063 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 064: 1 distinct dependencies below the limit.
mod source_064 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 065: 2 distinct dependencies below the limit.
mod source_065 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 066: 3 distinct dependencies below the limit.
mod source_066 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
    }
}

// Case 067: 4 distinct dependencies below the limit.
mod source_067 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 068: 5 distinct dependencies below the limit.
mod source_068 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 069: 6 distinct dependencies below the limit.
mod source_069 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
    }
}

// Case 070: 7 distinct dependencies below the limit.
mod source_070 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
        crate::target_1::call(); // A repeated edge still counts once.
    }
}

// Case 071: 1 distinct dependencies below the limit.
mod source_071 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 072: 2 distinct dependencies below the limit.
mod source_072 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 073: 3 distinct dependencies below the limit.
mod source_073 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
    }
}

// Case 074: 4 distinct dependencies below the limit.
mod source_074 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 075: 5 distinct dependencies below the limit.
mod source_075 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 076: 6 distinct dependencies below the limit.
mod source_076 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
    }
}

// Case 077: 7 distinct dependencies below the limit.
mod source_077 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 078: 1 distinct dependencies below the limit.
mod source_078 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 079: 2 distinct dependencies below the limit.
mod source_079 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 080: 3 distinct dependencies below the limit.
mod source_080 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_1::call(); // A repeated edge still counts once.
    }
}

// Case 081: 4 distinct dependencies below the limit.
mod source_081 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 082: 5 distinct dependencies below the limit.
mod source_082 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 083: 6 distinct dependencies below the limit.
mod source_083 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
    }
}

// Case 084: 7 distinct dependencies below the limit.
mod source_084 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 085: 1 distinct dependencies below the limit.
mod source_085 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 086: 2 distinct dependencies below the limit.
mod source_086 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 087: 3 distinct dependencies below the limit.
mod source_087 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
    }
}

// Case 088: 4 distinct dependencies below the limit.
mod source_088 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 089: 5 distinct dependencies below the limit.
mod source_089 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 090: 6 distinct dependencies below the limit.
mod source_090 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_1::call(); // A repeated edge still counts once.
    }
}

// Case 091: reduced orchestration shape with 7 collaborators.
mod source_091 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 092: reduced orchestration shape with 1 collaborators.
mod source_092 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 093: reduced orchestration shape with 2 collaborators.
mod source_093 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
    }
}

// Case 094: reduced orchestration shape with 3 collaborators.
mod source_094 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
    }
}

// Case 095: reduced orchestration shape with 4 collaborators.
mod source_095 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
    }
}

// Case 096: reduced orchestration shape with 5 collaborators.
mod source_096 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
    }
}

// Case 097: reduced orchestration shape with 6 collaborators.
mod source_097 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
    }
}

// Case 098: reduced orchestration shape with 7 collaborators.
mod source_098 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_3::call();
        crate::target_4::call();
        crate::target_5::call();
        crate::target_6::call();
        crate::target_7::call();
    }
}

// Case 099: reduced orchestration shape with 1 collaborators.
mod source_099 {
    pub fn run() {
        crate::target_1::call();
    }
}

// Case 100: reduced orchestration shape with 2 collaborators.
mod source_100 {
    pub fn run() {
        crate::target_1::call();
        crate::target_2::call();
        crate::target_1::call(); // A repeated edge still counts once.
    }
}

fn main() {}
