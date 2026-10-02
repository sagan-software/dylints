#![allow(dead_code)]

use std::string::String as StdString;

struct Date;
type DateText = std::string::String;
type DateStr = str;
struct DateTextWrapper(String);

struct Employee<'a> {
    dob: &'a str,
    hire_date: String,
    birth_date: DateText,
    review_date: std::string::String,
    renewal_date: &'a DateStr,
    approved_date: StdString,
    date_label: String,
    typed_birth_date: Date,
    wrapped_birth_date: DateTextWrapper,
}

fn main() {}
