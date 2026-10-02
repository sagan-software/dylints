# hardcoded_path

## What it does

Checks for string literals that hold a machine-specific filesystem path. It
recognizes Unix absolute paths under host root directories. Examples include
`/home/alice/cache` and `/Users/alice`, home paths such as `~/cache` or
`~alice/cache`, Windows drive paths such as `C:\Users`, and UNC paths such as
`\\server\share`.

## Why is this bad?

The code hardcodes a location on one machine. Builds, tests, and tools that use
it fail on other machines, in CI, and in containers.

## Known problems

The lint reads only the literal's text, not how code uses the value. A Unix path
warns only when its first component is a standard top-level directory: `bin`,
`boot`, `dev`, `etc`, `home`, `lib`, `lib64`, `media`, `mnt`, `nix`, `opt`,
`private`, `proc`, `root`, `run`, `sbin`, `snap`, `srv`, `sys`, `tmp`, `usr`,
`var`, `Applications`, `Library`, `System`, `Users`, or `Volumes`. It therefore
skips URL routes such as `"/api/users"` and `"/users/:id"` and comment text such
as `"// note"`, but it also skips a real path under another root, such as
`/data/cache`. It warns on fixed system paths such as `/dev/null`.

It does not flag relative paths such as `assets/icon.png` or `../src/lib.rs`,
URLs such as `https://example.com/assets`, byte strings, or paths built at
runtime.

## Example

```rust
fn cache_dir() -> &'static str {
    "/home/alice/.cache/example"
}
```

## Use instead

```rust
fn cache_dir() -> &'static str {
    "target/example-cache"
}
```
