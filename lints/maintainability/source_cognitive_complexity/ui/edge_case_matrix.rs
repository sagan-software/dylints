// compile-flags: --edition 2024

#![allow(dead_code)]

// This matrix keeps every case below all three callable-metric limits.
// Each case is source-authored so the lint walks real HIR rather than macro output.

// Case 001: 1 sequential decisions remain independently measurable.
fn flat_01(flags: [bool; 7]) -> usize {
    let mut hits = 0;
    if flags[0] {
        hits += 1;
    }
    hits
}

// Case 002: 2 sequential decisions remain independently measurable.
fn flat_02(flags: [bool; 7]) -> usize {
    let mut hits = 0;
    if flags[0] {
        hits += 1;
    }
    if flags[1] {
        hits += 1;
    }
    hits
}

// Case 003: 3 sequential decisions remain independently measurable.
fn flat_03(flags: [bool; 7]) -> usize {
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

// Case 004: 4 sequential decisions remain independently measurable.
fn flat_04(flags: [bool; 7]) -> usize {
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
    if flags[3] {
        hits += 1;
    }
    hits
}

// Case 005: 5 sequential decisions remain independently measurable.
fn flat_05(flags: [bool; 7]) -> usize {
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
    if flags[3] {
        hits += 1;
    }
    if flags[4] {
        hits += 1;
    }
    hits
}

// Case 006: 6 sequential decisions remain independently measurable.
fn flat_06(flags: [bool; 7]) -> usize {
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
    if flags[3] {
        hits += 1;
    }
    if flags[4] {
        hits += 1;
    }
    if flags[5] {
        hits += 1;
    }
    hits
}

// Case 007: 7 sequential decisions remain independently measurable.
fn flat_07(flags: [bool; 7]) -> usize {
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
    if flags[3] {
        hits += 1;
    }
    if flags[4] {
        hits += 1;
    }
    if flags[5] {
        hits += 1;
    }
    if flags[6] {
        hits += 1;
    }
    hits
}

// Case 008: 1 sequential decisions remain independently measurable.
fn flat_08(flags: [bool; 7]) -> usize {
    let mut hits = 0;
    if flags[0] {
        hits += 1;
    }
    hits
}

// Case 009: 2 sequential decisions remain independently measurable.
fn flat_09(flags: [bool; 7]) -> usize {
    let mut hits = 0;
    if flags[0] {
        hits += 1;
    }
    if flags[1] {
        hits += 1;
    }
    hits
}

// Case 010: 3 sequential decisions remain independently measurable.
fn flat_10(flags: [bool; 7]) -> usize {
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

// Case 011: 4 sequential decisions remain independently measurable.
fn flat_11(flags: [bool; 7]) -> usize {
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
    if flags[3] {
        hits += 1;
    }
    hits
}

// Case 012: 5 sequential decisions remain independently measurable.
fn flat_12(flags: [bool; 7]) -> usize {
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
    if flags[3] {
        hits += 1;
    }
    if flags[4] {
        hits += 1;
    }
    hits
}

// Case 013: 6 sequential decisions remain independently measurable.
fn flat_13(flags: [bool; 7]) -> usize {
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
    if flags[3] {
        hits += 1;
    }
    if flags[4] {
        hits += 1;
    }
    if flags[5] {
        hits += 1;
    }
    hits
}

// Case 014: 7 sequential decisions remain independently measurable.
fn flat_14(flags: [bool; 7]) -> usize {
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
    if flags[3] {
        hits += 1;
    }
    if flags[4] {
        hits += 1;
    }
    if flags[5] {
        hits += 1;
    }
    if flags[6] {
        hits += 1;
    }
    hits
}

// Case 015: 1 sequential decisions remain independently measurable.
fn flat_15(flags: [bool; 7]) -> usize {
    let mut hits = 0;
    if flags[0] {
        hits += 1;
    }
    hits
}

// Case 016: a 2-term short-circuit chain stays bounded.
fn boolean_chain_01(flags: [bool; 7]) -> bool {
    flags[0] && flags[1]
}

// Case 017: a 3-term short-circuit chain stays bounded.
fn boolean_chain_02(flags: [bool; 7]) -> bool {
    flags[0] || flags[1] || flags[2]
}

// Case 018: a 4-term short-circuit chain stays bounded.
fn boolean_chain_03(flags: [bool; 7]) -> bool {
    flags[0] && flags[1] && flags[2] && flags[3]
}

// Case 019: a 5-term short-circuit chain stays bounded.
fn boolean_chain_04(flags: [bool; 7]) -> bool {
    flags[0] || flags[1] || flags[2] || flags[3] || flags[4]
}

// Case 020: a 6-term short-circuit chain stays bounded.
fn boolean_chain_05(flags: [bool; 7]) -> bool {
    flags[0] && flags[1] && flags[2] && flags[3] && flags[4] && flags[5]
}

// Case 021: a 7-term short-circuit chain stays bounded.
fn boolean_chain_06(flags: [bool; 7]) -> bool {
    flags[0] || flags[1] || flags[2] || flags[3] || flags[4] || flags[5] || flags[6]
}

// Case 022: a 2-term short-circuit chain stays bounded.
fn boolean_chain_07(flags: [bool; 7]) -> bool {
    flags[0] && flags[1]
}

// Case 023: a 3-term short-circuit chain stays bounded.
fn boolean_chain_08(flags: [bool; 7]) -> bool {
    flags[0] || flags[1] || flags[2]
}

// Case 024: a 4-term short-circuit chain stays bounded.
fn boolean_chain_09(flags: [bool; 7]) -> bool {
    flags[0] && flags[1] && flags[2] && flags[3]
}

// Case 025: a 5-term short-circuit chain stays bounded.
fn boolean_chain_10(flags: [bool; 7]) -> bool {
    flags[0] || flags[1] || flags[2] || flags[3] || flags[4]
}

// Case 026: a 6-term short-circuit chain stays bounded.
fn boolean_chain_11(flags: [bool; 7]) -> bool {
    flags[0] && flags[1] && flags[2] && flags[3] && flags[4] && flags[5]
}

// Case 027: a 7-term short-circuit chain stays bounded.
fn boolean_chain_12(flags: [bool; 7]) -> bool {
    flags[0] || flags[1] || flags[2] || flags[3] || flags[4] || flags[5] || flags[6]
}

// Case 028: a 2-term short-circuit chain stays bounded.
fn boolean_chain_13(flags: [bool; 7]) -> bool {
    flags[0] && flags[1]
}

// Case 029: a 3-term short-circuit chain stays bounded.
fn boolean_chain_14(flags: [bool; 7]) -> bool {
    flags[0] || flags[1] || flags[2]
}

// Case 030: a 4-term short-circuit chain stays bounded.
fn boolean_chain_15(flags: [bool; 7]) -> bool {
    flags[0] && flags[1] && flags[2] && flags[3]
}

// Case 031: a 2-alternative match adds alternatives instead of multiplying them.
fn match_01(value: u8) -> u8 {
    match value {
        0 => 0,
        _ => 2,
    }
}

// Case 032: a 3-alternative match adds alternatives instead of multiplying them.
fn match_02(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 1,
        _ => 3,
    }
}

// Case 033: a 4-alternative match adds alternatives instead of multiplying them.
fn match_03(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 1,
        2 => 2,
        _ => 4,
    }
}

// Case 034: a 5-alternative match adds alternatives instead of multiplying them.
fn match_04(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        _ => 5,
    }
}

// Case 035: a 6-alternative match adds alternatives instead of multiplying them.
fn match_05(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        _ => 6,
    }
}

// Case 036: a 7-alternative match adds alternatives instead of multiplying them.
fn match_06(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 5,
        _ => 7,
    }
}

// Case 037: a 2-alternative match adds alternatives instead of multiplying them.
fn match_07(value: u8) -> u8 {
    match value {
        0 => 0,
        _ => 2,
    }
}

// Case 038: a 3-alternative match adds alternatives instead of multiplying them.
fn match_08(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 1,
        _ => 3,
    }
}

// Case 039: a 4-alternative match adds alternatives instead of multiplying them.
fn match_09(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 1,
        2 => 2,
        _ => 4,
    }
}

// Case 040: a 5-alternative match adds alternatives instead of multiplying them.
fn match_10(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        _ => 5,
    }
}

// Case 041: a 6-alternative match adds alternatives instead of multiplying them.
fn match_11(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        _ => 6,
    }
}

// Case 042: a 7-alternative match adds alternatives instead of multiplying them.
fn match_12(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 5,
        _ => 7,
    }
}

// Case 043: a 2-alternative match adds alternatives instead of multiplying them.
fn match_13(value: u8) -> u8 {
    match value {
        0 => 0,
        _ => 2,
    }
}

// Case 044: a 3-alternative match adds alternatives instead of multiplying them.
fn match_14(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 1,
        _ => 3,
    }
}

// Case 045: a 4-alternative match adds alternatives instead of multiplying them.
fn match_15(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 1,
        2 => 2,
        _ => 4,
    }
}

// Case 046: 1 loop stages do not inherit compiler desugaring.
fn loops_01(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    hits
}

// Case 047: 2 loop stages do not inherit compiler desugaring.
fn loops_02(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    while flags[1] && hits == 0 {
        hits += 1;
    }
    hits
}

// Case 048: 3 loop stages do not inherit compiler desugaring.
fn loops_03(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    while flags[1] && hits == 0 {
        hits += 1;
    }
    for flag in [flags[2]] {
        if flag {
            hits += 1;
        }
    }
    hits
}

// Case 049: 4 loop stages do not inherit compiler desugaring.
fn loops_04(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    while flags[1] && hits == 0 {
        hits += 1;
    }
    for flag in [flags[2]] {
        if flag {
            hits += 1;
        }
    }
    while flags[3] && hits == 0 {
        hits += 1;
    }
    hits
}

// Case 050: 1 loop stages do not inherit compiler desugaring.
fn loops_05(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    hits
}

// Case 051: 2 loop stages do not inherit compiler desugaring.
fn loops_06(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    while flags[1] && hits == 0 {
        hits += 1;
    }
    hits
}

// Case 052: 3 loop stages do not inherit compiler desugaring.
fn loops_07(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    while flags[1] && hits == 0 {
        hits += 1;
    }
    for flag in [flags[2]] {
        if flag {
            hits += 1;
        }
    }
    hits
}

// Case 053: 4 loop stages do not inherit compiler desugaring.
fn loops_08(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    while flags[1] && hits == 0 {
        hits += 1;
    }
    for flag in [flags[2]] {
        if flag {
            hits += 1;
        }
    }
    while flags[3] && hits == 0 {
        hits += 1;
    }
    hits
}

// Case 054: 1 loop stages do not inherit compiler desugaring.
fn loops_09(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    hits
}

// Case 055: 2 loop stages do not inherit compiler desugaring.
fn loops_10(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    while flags[1] && hits == 0 {
        hits += 1;
    }
    hits
}

// Case 056: 3 loop stages do not inherit compiler desugaring.
fn loops_11(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    while flags[1] && hits == 0 {
        hits += 1;
    }
    for flag in [flags[2]] {
        if flag {
            hits += 1;
        }
    }
    hits
}

// Case 057: 4 loop stages do not inherit compiler desugaring.
fn loops_12(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    while flags[1] && hits == 0 {
        hits += 1;
    }
    for flag in [flags[2]] {
        if flag {
            hits += 1;
        }
    }
    while flags[3] && hits == 0 {
        hits += 1;
    }
    hits
}

// Case 058: 1 loop stages do not inherit compiler desugaring.
fn loops_13(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    hits
}

// Case 059: 2 loop stages do not inherit compiler desugaring.
fn loops_14(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    while flags[1] && hits == 0 {
        hits += 1;
    }
    hits
}

// Case 060: 3 loop stages do not inherit compiler desugaring.
fn loops_15(flags: [bool; 4]) -> usize {
    let mut hits = 0;
    for flag in [flags[0]] {
        if flag {
            hits += 1;
        }
    }
    while flags[1] && hits == 0 {
        hits += 1;
    }
    for flag in [flags[2]] {
        if flag {
            hits += 1;
        }
    }
    hits
}

// Case 061: nesting depth 1 stays distinct from a flat decision count.
fn nested_01(flags: [bool; 4]) -> bool {
    if flags[0] {
        return true;
    }
    false
}

// Case 062: nesting depth 2 stays distinct from a flat decision count.
fn nested_02(flags: [bool; 4]) -> bool {
    if flags[0] {
        if flags[1] {
            return true;
        }
    }
    false
}

// Case 063: nesting depth 3 stays distinct from a flat decision count.
fn nested_03(flags: [bool; 4]) -> bool {
    if flags[0] {
        if flags[1] {
            if flags[2] {
                return true;
            }
        }
    }
    false
}

// Case 064: nesting depth 4 stays distinct from a flat decision count.
fn nested_04(flags: [bool; 4]) -> bool {
    if flags[0] {
        if flags[1] {
            if flags[2] {
                if flags[3] {
                    return true;
                }
            }
        }
    }
    false
}

// Case 065: nesting depth 1 stays distinct from a flat decision count.
fn nested_05(flags: [bool; 4]) -> bool {
    if flags[0] {
        return true;
    }
    false
}

// Case 066: nesting depth 2 stays distinct from a flat decision count.
fn nested_06(flags: [bool; 4]) -> bool {
    if flags[0] {
        if flags[1] {
            return true;
        }
    }
    false
}

// Case 067: nesting depth 3 stays distinct from a flat decision count.
fn nested_07(flags: [bool; 4]) -> bool {
    if flags[0] {
        if flags[1] {
            if flags[2] {
                return true;
            }
        }
    }
    false
}

// Case 068: nesting depth 4 stays distinct from a flat decision count.
fn nested_08(flags: [bool; 4]) -> bool {
    if flags[0] {
        if flags[1] {
            if flags[2] {
                if flags[3] {
                    return true;
                }
            }
        }
    }
    false
}

// Case 069: nesting depth 1 stays distinct from a flat decision count.
fn nested_09(flags: [bool; 4]) -> bool {
    if flags[0] {
        return true;
    }
    false
}

// Case 070: nesting depth 2 stays distinct from a flat decision count.
fn nested_10(flags: [bool; 4]) -> bool {
    if flags[0] {
        if flags[1] {
            return true;
        }
    }
    false
}

// Case 071: nesting depth 3 stays distinct from a flat decision count.
fn nested_11(flags: [bool; 4]) -> bool {
    if flags[0] {
        if flags[1] {
            if flags[2] {
                return true;
            }
        }
    }
    false
}

// Case 072: nesting depth 4 stays distinct from a flat decision count.
fn nested_12(flags: [bool; 4]) -> bool {
    if flags[0] {
        if flags[1] {
            if flags[2] {
                if flags[3] {
                    return true;
                }
            }
        }
    }
    false
}

// Case 073: nesting depth 1 stays distinct from a flat decision count.
fn nested_13(flags: [bool; 4]) -> bool {
    if flags[0] {
        return true;
    }
    false
}

// Case 074: nesting depth 2 stays distinct from a flat decision count.
fn nested_14(flags: [bool; 4]) -> bool {
    if flags[0] {
        if flags[1] {
            return true;
        }
    }
    false
}

// Case 075: nesting depth 3 stays distinct from a flat decision count.
fn nested_15(flags: [bool; 4]) -> bool {
    if flags[0] {
        if flags[1] {
            if flags[2] {
                return true;
            }
        }
    }
    false
}

// Case 076: a nested callable owns its 1 decisions.
fn closure_01(flags: [bool; 7]) -> usize {
    let evaluate = || {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

// Case 077: a nested callable owns its 2 decisions.
fn closure_02(flags: [bool; 7]) -> usize {
    let evaluate = || {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

// Case 078: a nested callable owns its 3 decisions.
fn closure_03(flags: [bool; 7]) -> usize {
    let evaluate = || {
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
    };
    evaluate()
}

// Case 079: a nested callable owns its 4 decisions.
fn closure_04(flags: [bool; 7]) -> usize {
    let evaluate = || {
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
        if flags[3] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

// Case 080: a nested callable owns its 5 decisions.
fn closure_05(flags: [bool; 7]) -> usize {
    let evaluate = || {
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
        if flags[3] {
            hits += 1;
        }
        if flags[4] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

// Case 081: a nested callable owns its 6 decisions.
fn closure_06(flags: [bool; 7]) -> usize {
    let evaluate = || {
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
        if flags[3] {
            hits += 1;
        }
        if flags[4] {
            hits += 1;
        }
        if flags[5] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

// Case 082: a nested callable owns its 7 decisions.
fn closure_07(flags: [bool; 7]) -> usize {
    let evaluate = || {
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
        if flags[3] {
            hits += 1;
        }
        if flags[4] {
            hits += 1;
        }
        if flags[5] {
            hits += 1;
        }
        if flags[6] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

// Case 083: a nested callable owns its 1 decisions.
fn closure_08(flags: [bool; 7]) -> usize {
    let evaluate = || {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

// Case 084: a nested callable owns its 2 decisions.
fn closure_09(flags: [bool; 7]) -> usize {
    let evaluate = || {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        if flags[1] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

// Case 085: a nested callable owns its 3 decisions.
fn closure_10(flags: [bool; 7]) -> usize {
    let evaluate = || {
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
    };
    evaluate()
}

// Case 086: a nested callable owns its 4 decisions.
fn closure_11(flags: [bool; 7]) -> usize {
    let evaluate = || {
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
        if flags[3] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

// Case 087: a nested callable owns its 5 decisions.
fn closure_12(flags: [bool; 7]) -> usize {
    let evaluate = || {
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
        if flags[3] {
            hits += 1;
        }
        if flags[4] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

// Case 088: a nested callable owns its 6 decisions.
fn closure_13(flags: [bool; 7]) -> usize {
    let evaluate = || {
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
        if flags[3] {
            hits += 1;
        }
        if flags[4] {
            hits += 1;
        }
        if flags[5] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

// Case 089: a nested callable owns its 7 decisions.
fn closure_14(flags: [bool; 7]) -> usize {
    let evaluate = || {
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
        if flags[3] {
            hits += 1;
        }
        if flags[4] {
            hits += 1;
        }
        if flags[5] {
            hits += 1;
        }
        if flags[6] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

// Case 090: a nested callable owns its 1 decisions.
fn closure_15(flags: [bool; 7]) -> usize {
    let evaluate = || {
        let mut hits = 0;
        if flags[0] {
            hits += 1;
        }
        hits
    };
    evaluate()
}

fn main() {}
