# Usage

`only` is a deterministic, cross-platform task runner driven by an `Onlyfile`.

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
!verbose true
!shell deno          # default cross-platform shell
```

### Tasks and Parameters

```text
build():
    cargo build --release

serve(port="3000", host="127.0.0.1"):
    echo "Serving on {{host}}:{{port}}"
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
!verbose true

% Run checks.
check() ? @has("cargo"):
    cargo check
    cargo fmt --all --check
    cargo clippy --workspace -- -D warnings
    
check():
    echo "Cannot found cargo."

% Run tests.
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test

% Run full CI.
ci() & check & test:
    echo "CI complete!"

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
