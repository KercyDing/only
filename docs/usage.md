# Usage

`only` is a deterministic, cross-platform task runner driven by an `Onlyfile`.

The CLI runs on top of a staged frontend pipeline:

```text
source -> syntax -> semantic -> engine -> cli
```

`only` treats `Onlyfile` as a language with distinct syntax, semantic, and execution stages. That keeps runtime behavior, diagnostics, and future tooling aligned on the same model.

This repository itself is a Cargo workspace. When installing the local `only` binary from a clone
of this repo, target the CLI package directory instead of the workspace root:

```bash
cargo install --path crates/cli --force
```

## File Discovery

By default, `only` looks for `Onlyfile` or `onlyfile` in the current directory and all parent directories until the filesystem root is reached.

You can specify a file explicitly:

```bash
only -f ./examples/Onlyfile
```

Print the resolved `Onlyfile` path without running any task:

```bash
only -p
```

## CLI Basics

```bash
only --help          # Show help
only                 # List all available tasks
only check           # Run a global task
only test            # Run the test task
only dev             # Show namespace help
only dev build       # Run a namespaced task
only rel run
```

## Current Capabilities

Current user-facing behavior includes:

- automatic `Onlyfile` discovery from the current directory upward
- dynamic task listing and namespace-aware help
- directives, doc comments, namespaces, helper tasks, and task declarations
- parameter defaults and `{{name}}` interpolation
- dependency chaining with `&`, including parallel groups via `(a, b)`
- guards such as `@os`, `@arch`, `@env`, and `@has`
- `shell?=` host shell preference with fallback
- `!preview true|false` command previews before execution
- semantic validation before execution, including duplicate names and undefined references

## Why This Structure Matters

`only` is not built as a CLI that happens to parse a file. It is built as a language pipeline that happens to power a CLI today.

That difference matters:

- terminal diagnostics can stay readable without coupling parsing logic to output rendering
- editor features can reuse syntax and semantic analysis instead of rebuilding ad hoc parsers
- future web tooling can consume the same CST, AST, and symbol information as the CLI
- runtime changes stay isolated in `engine` instead of leaking into parsing and validation

Compared with tools centered on shell execution or YAML orchestration, this structure gives `only` more room to grow without turning the implementation into a monolith.

Override parameters:

```bash
only --set name="0.0.0.0" serve
```

## Onlyfile Structure

An `Onlyfile` consists of:
- optional top-level directives (`!`)
- global tasks
- optional `[namespace]` sections

## Syntax

### Doc Comments

Lines starting with `%` document the following top-level declaration:

```text
% Format the codebase.
fmt():
    cargo fmt --all
```

The same rule applies to namespaces:

```text
% Developer workflow.
[dev]

% Run the default workflow.
workflow():
    cargo run
```

### Directives

```text
!echo true
!preview false
!shell deno          # default cross-platform shell
```

- `!echo true|false` controls whether task output is streamed on success
- `!preview true|false` prints the selected task variant and rendered commands before execution
- `!shell <name>` sets the default shell for tasks in the file

### Tasks and Parameters

```text
build():
    cargo build --release

serve(port="3000", host="127.0.0.1"):
    echo "Serving on {{host}}:{{port}}"
```

Task names beginning with `_` are helper tasks. They can be used as dependencies, but cannot be invoked directly and are hidden from normal task listings.

```text
_prepare():
    cargo fmt --all --check

ci() & _prepare:
    cargo test
```

### Smart Shell Selection

Use `shell?=` to prefer a specific shell with automatic fallback:

```text
build() ? @os("windows") shell?=pwsh:
    Get-ChildItem -Force
```

### Dependencies

Use `&` for serial dependencies.

```text
ci() & fmt & check & test:
    echo "CI complete"
```

Use parentheses to run a dependency group in parallel after the previous serial stage finishes.

```text
release() & build & (package, publish):
    echo "Release complete"
```

In that example, `build` runs first. After it succeeds, `package` and `publish` run in parallel. The `release` task runs after both finish.

### Guards

```text
build() ? @os("linux"):
    cargo build
```

Supported probes:
- `@os("linux")` / `@os("macos")` / `@os("windows")`
- `@arch("x86_64")` / `@arch("aarch64")`
- `@env("CI")`
- `@has("cargo")` / `@has("pwsh")`

### Namespaces

```text
% Development builds.
[dev]
build():
    cargo build

run():
    cargo run
```

Run with:

```bash
only dev build
only dev run
```

### Practical Example

```text
!echo true
!preview true

% Run checks.
check() ? @has("cargo"):
    cargo check
    cargo fmt --all --check
    cargo clippy --workspace -- -D warnings
    
check():
    echo "cargo not found, skipping checks"

% Run tests.
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test

% Internal helper reused by install on Windows.
_release_build():
    cargo build --release

% Install the local binary.
install() ? @os("windows") & _release_build shell?=pwsh:
    Write-Output "Windows: cannot replace running binary. Run:`n  Copy-Item target/release/only.exe -Destination `$env:USERPROFILE\.cargo\bin\ -Force"

install():
    cargo install --path crates/cli --force

% Run full CI.
ci() & check & test:
    echo "CI complete!"

% Build first, then fan out packaging work.
release() & build & (package, publish):
    echo "Release complete!"

% Development builds.
[dev]
build():
    cargo build

run():
    cargo run

% Release builds.
[rel]
build():
    cargo build --release

run():
    cargo run --release

test() ? @has("cargo-nextest"):
    cargo nextest run --release

test():
    cargo test --release
```

Usage:

```bash
only
only ci
only dev build
only rel run
only rel test
```
