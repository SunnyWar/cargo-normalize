# cargo-normalize

`cargo-normalize` is a Cargo subcommand that enforces vertical ordering for Rust source files.

`rustfmt` is excellent at whitespace and line-level formatting, but it intentionally does not reorder top-level items. `cargo-normalize` fills that gap by normalizing module structure into a predictable layout.

## What It Solves

Rust modules often drift into mixed ordering over time: imports scattered between declarations, impl blocks detached from their types, tests interleaved with production code, etc.

`cargo-normalize` parses Rust files and rewrites them into a stable, configurable order so structure stays consistent across contributors and code reviews.

Idiomatic ordering:

1. Crate Attributes (`#![warn(...)]`, `#![no_std]`) - Must be at the very top.
2. Imports (`use`) - Sorted by `rustfmt` (Std, external, then local).
3. Modules (`mod`) - Defines the tree structure before code.
4. Macros (`macro_rules!`) - Should be defined before use (unless exported).
5. Types and Aliases (`type`, `const`, `static`) - Establishes file vocabulary.
6. Data Structures (`enum`, `struct`, `union`) - Core domain definitions.
7. Implementations (`impl`) - Inherent impls, then trait impls.
8. Traits (`trait`) - New abstractions near consuming types.
9. FFI (`extern`) - Typically near the bottom or isolated in a module.
10. Free Functions (`fn`) - Helpers and pure logic.
11. Tests (`#[cfg(test)]`) - Always at the bottom.

## Features

- Predictable top-level item ordering
- Check mode for CI/CD (`--check`)
- Comment preservation
- Configurable ordering via `normalize.toml`
- Support for idiomatic priority groups (attributes, imports, modules, macros, constants, types, data structures, impls, traits, extern blocks, functions, tests)

## Installation

```bash
cargo install cargo-normalize
```

## Usage

Default mode (no options): check all normalization features, no writes.

```bash
cargo normalize
```

Move all normalization features:

```bash
cargo normalize --all
```

Move specific features (repeat `--move-feature` as needed):

```bash
cargo normalize --move-feature modules
cargo normalize --move-feature constants --move-feature functions
```

Explicit check mode (no writes; ideal for CI):

```bash
cargo normalize --check
```

Show command-line options:

```bash
cargo normalize --help
cargo normalize help
```

Available values for `--move-feature`:

- `attributes`
- `imports`
- `modules`
- `macros`
- `constants`
- `types`
- `enums`
- `structs`
- `impls`
- `traits`
- `extern_blocks`
- `functions`
- `tests`

## Configuration

Create a `normalize.toml` in your crate root to customize item ordering and block compaction.

Example:

```toml
# normalize.toml
priority = [
  "attributes",
  "imports",
  "modules",
  "macros",
  "constants",
  "types",
  "enums",
  "structs",
  "impls",
  "traits",
  "extern_blocks",
  "functions",
  "tests"
]
compact_use_block = true
compact_const_block = true
compact_mod_block = true
```

Move "modules" above or below "macros" in priority to control their relative placement.
Set `compact_mod_block = false` if you prefer a blank line between consecutive `mod` declarations.
Move "constants" above or below "types" in priority to control their relative placement.

## CI Integration

Minimal GitHub Actions example:

```yaml
name: normalize
on: [pull_request]

jobs:
  check-normalization:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-normalize
      - run: cargo normalize --check
```

## Scope

`cargo-normalize` complements `rustfmt`; it does not replace it.

Recommended workflow:

1. Run `cargo normalize`
2. Run `cargo fmt`
3. Commit
