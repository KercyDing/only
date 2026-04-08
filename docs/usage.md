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
only test            # Run a global task
only dev             # Show namespace help
only dev smoke       # Run a namespaced task
only dev smoke "hello world"
```

Override parameters:

```bash
only --set name="hello world" dev smoke
```

## Onlyfile Structure

An `Onlyfile` consists of:
- optional top-level directives (`!`)
- global tasks
- optional `[namespace]` sections

## Syntax

### Doc Comments

Lines starting with `%` document the following task:

```text
% Format the codebase.
fmt():
    cargo fmt --all
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

### Variadic Parameters

The last parameter can be variadic:

```text
fmt(..flags):
    cargo fmt --all {{flags}}
```

### Smart Shell Selection

Use `shell?=` to prefer a specific shell with automatic fallback:

```text
build() ? @os("windows") shell?=pwsh:
    Get-ChildItem -Force
```

### Dependencies

Use `&` for serial and `|` for parallel dependencies. Parentheses are supported.

```text
ci() & fmt & check & test:
    echo "CI complete"

all() (build() | lint()) & test() & deploy():
    echo "All done"
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
[dev]
workflow() & fmt & test:
    echo "dev complete"

smoke(name="It's a smoke command"):
    echo "{{name}}"
```

Run with:

```bash
only dev workflow
only dev smoke "hello"
```

### Practical Example

```text
!verbose true

% Format the Rust codebase.
fmt():
    cargo fmt --all

% Run checks.
check() ? @has("cargo"):
    cargo check
    cargo fmt --all --check
    cargo clippy --workspace -- -D warnings
    
check():
    echo "Cannot found cargo."

% Run tests.
test():
    cargo test

% Run full CI.
ci() & fmt & check & test:
    echo "✅ CI complete!"

[dev]
% Developer workflow.
workflow() & fmt & test:
    echo "dev complete!"

% Run a namespaced smoke command.
smoke(name="It's a smoke command"):
    echo "{{name}}"
```

Usage:

```bash
only
only ci
only dev workflow
only dev smoke "custom message"
```
