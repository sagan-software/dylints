# owned_input_field_clones

## What it does

Checks functions whose tail expression is a struct literal with fields cloned
from a parameter, such as `id: input.id.clone()`. It warns when:

- an owned parameter has two or more fields cloned and no field moved out of
  it, or
- a borrowed parameter has at least one field cloned.

## Why is this bad?

When the function owns the input, each clone allocates a copy of data that
could be moved, and the original is dropped right after. When the function
borrows the input but always needs owned fields, the signature hides that
cost from callers, who may already have an owned value to give up.

## Known problems

The borrowed rule includes `&self`. Any method that clones one field of `self`
into a returned struct, such as `fn summary(&self) -> Summary`, triggers the
lint, even when taking ownership is not an option.

Clones of `Copy` fields and cheap clones such as `Arc` count toward the total.

Only a struct literal in tail position is checked. A struct inside `Ok(..)`,
a `return` statement, or a struct built with `..base` syntax is not checked.
Destructured parameters are not checked.

## Example

```rust
struct Input {
    id: String,
    name: String,
}

struct Output {
    id: String,
    name: String,
}

fn convert(input: Input) -> Output {
    Output {
        id: input.id.clone(),
        name: input.name.clone(),
    }
}
```

## Use instead

```rust
struct Input {
    id: String,
    name: String,
}

struct Output {
    id: String,
    name: String,
}

fn convert(input: Input) -> Output {
    Output {
        id: input.id,
        name: input.name,
    }
}
```
