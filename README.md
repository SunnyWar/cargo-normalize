# cargo-normalize

`cargo-normalize` is a Cargo subcommand that enforces vertical ordering for Rust source files.

`rustfmt` is excellent at whitespace and line-level formatting, but it intentionally does not reorder top-level items. `cargo-normalize` fills that gap by normalizing module structure into a predictable layout.

## What It Solves

Rust modules often drift into mixed ordering over time: imports scattered between declarations, impl blocks detached from their types, tests interleaved with production code, etc.

`cargo-normalize` parses Rust files and rewrites them into a stable, configurable order so structure stays consistent across contributors and code reviews.

Default ordering:

1. Imports (`use`)
2. Constants (`const`, `static`)
3. Enums
4. Structs
5. Impls (grouped with their corresponding structs)
6. Traits
7. Tests (`#[cfg(test)]`, moved to the bottom)

## Features

- Predictable Layout
  - Moves imports to the top of each module.
  - Pushes test modules to the bottom.
  - Groups `impl` blocks with their owning structs to keep type definitions and behavior adjacent.
- Check Mode for CI/CD
  - Use `--check` to verify files are already normalized.
  - Exits non-zero when normalization changes would be required.
- Comment Preservation
  - Uses lossless parsing and high-fidelity printing to preserve doc comments and internal comments.
- Configurable Ordering
  - Supports project-level customization via `normalize.toml`.

## Installation

```bash
cargo install cargo-normalize
```

## Usage

Normalize the current crate:

```bash
cargo normalize
```

Check mode (no writes; ideal for CI):

```bash
cargo normalize --check
```

## Configuration

Create a `normalize.toml` in your crate root to define your preferred item order.

Example:

```toml
# normalize.toml
order = [
  "imports",
  "constants",
  "enums",
  "structs",
  "impls",
  "traits",
  "tests"
]
```

## Technical Notes

`cargo-normalize` is built on Rust syntax tooling designed for fidelity and correctness:

- `syn` for parsing Rust source into an AST
- `prettyplease` and/or `quote` for high-fidelity code generation

This combination allows structural reordering while preserving the original semantics and comments.

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
