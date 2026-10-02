# date_string_field

## What it does

Checks for named fields whose type is `String` or `&str` and whose name marks a
date. The names are `birthdate`, names containing the word `dob`, `birth_date`,
or `date_of_birth`, and any name ending in `_date`, compared without case.

## Why is this bad?

A string field accepts any text, so a value such as `"2024-02-30"` or
`"03/04/2024"` moves through the program unchecked. Each reader must parse it
again and may read the parts in a different order. A date type parses once, at
the boundary, and compares dates correctly.

## Known problems

The lint checks only the exact types `String` and `&str`, after resolving type
aliases and `use` renames. It does not flag `Option<String>`, `Vec<String>`,
`Box<str>`, or `Cow<'_, str>`.

It can warn on a field that holds a formatted date for display. It does not
flag other date names, such as `created_on` or `date_label`.

## Example

```rust
struct Profile<'a> {
    date_of_birth: String,
    renewal_date: &'a str,
}
```

## Use instead

```rust
use chrono::NaiveDate;

struct Profile {
    date_of_birth: NaiveDate,
    renewal_date: NaiveDate,
}
```
