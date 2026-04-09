# Only

[![crates.io](https://img.shields.io/crates/v/only.svg)](https://crates.io/crates/only)
[![license](https://img.shields.io/crates/l/only.svg)](LICENSE)

**One `Onlyfile`. One behavior. Every platform.**

Only is a cross-platform task runner built around a real task language.

Write tasks once, keep one execution model, and get predictable results on **macOS, Linux, and Windows**.

- **Cross-platform by default** — no Git Bash, no `if os()` hacks, no `platforms:` boilerplate
- **A better task language** — readable task syntax with parameters, guards, namespaces, and interpolation
- **Built for tooling** — the same core model can power execution, diagnostics, editor features, and future visual workflows

```Onlyfile
check():
    cargo check

test():
    cargo test

ci() & check & test:
    echo "CI complete"
```

Run `only`, `only check`, or `only ci`, and you're off.

---

## Why It Works 🧠

`only` treats `Onlyfile` as a real language, not just a thin wrapper around shell commands.

Parsing, validation, planning, and execution are kept as separate stages. That keeps terminal errors readable today and leaves room for editor tooling, language-server features, and future visual workflows without rebuilding the core model later.

The execution path is intentionally simple:

```text
source -> syntax -> semantic -> engine -> cli
```

In practice, that means one source of truth for task structure, diagnostics, interpolation, dependency planning, and host integrations.

---

## Quick Start ⚡

Create an `Onlyfile` in your project root:

```Onlyfile
!verbose true

% Run cargo check.
check():
    cargo check
    cargo fmt --all --check
    cargo clippy --workspace -- -D warnings

% Run the full test suite.
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test

% Run formatter, type checks, and tests.
ci() & check & test:
    echo "CI complete!"

[dev]
% Build in development mode.
build():
    cargo build

[rel]
% Build in release mode.
build():
    cargo build --release
```

Then run:

```shell
only                # list all tasks
only check
only test
only dev build
only rel build
```

You can also document a namespace by placing `%` on the line immediately above it:

```Onlyfile
% Developer workflow.
[dev]

% Build in development mode.
build():
    cargo build
```

### Advanced Example

```Onlyfile
!verbose true

% Run checks only if cargo is available.
check() ? @has("cargo"):
    cargo check
    cargo fmt --all --check
    cargo clippy --workspace -- -D warnings

check():
    echo "cargo not found, skipping checks"

% Prefer nextest when it is installed.
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test

% Install the local binary.
install() ? @os("windows") shell?=pwsh:
    cargo build --release
    Write-Output "Windows: cannot replace running binary. Run:`n  Copy-Item target/release/only.exe -Destination `$env:USERPROFILE\.cargo\bin\ -Force"

install():
    cargo install --path crates/cli --force

% Full CI pipeline.
ci() & check & test:
    echo "CI completed successfully"

% Development builds.
[dev]
% Build in development mode.
build():
    cargo build

% Release builds.
[rel]
% Build in release mode.
build():
    cargo build --release
```

---

## Why Only ✨

- **Actually cross-platform by default** — `deno_task_shell` keeps behavior aligned across macOS, Linux, and Windows
- **A better task language** — function-style signatures, parameters, defaults, guards, namespaces, and interpolation stay readable
- **Clear execution flow** — dependencies, planning, and runtime behavior are explicit instead of being buried in shell glue
- **Better diagnostics and help** — dynamic task listing and structured validation make the terminal experience less guessy
- **Built for tooling, not just execution** — the same pipeline can power CLI, editor features, language servers, and future visual workflows

| Tool | Best fit | Core model | Portability | Tooling headroom |
|------|----------|------------|-------------|------------------|
| `only` | tasks that should stay simple now and grow later | task language | consistent by default | high |
| `just` | straightforward command running | command runner | shell-sensitive in practice | medium |
| `taskfile` | config-heavy orchestration | YAML orchestration | workable, but heavier | medium |

`only` is for the case where you want both a pleasant task authoring experience and a format that can grow into real tooling without being redesigned later.

---

## Installation 📦

Published release:

```shell
cargo install only
```

Latest GitHub version:

```shell
cargo install --git https://github.com/KercyDing/only only
```

Local workspace:

```shell
cargo install --path crates/cli --force
```

---

## Docs 📚

- Usage and syntax: **[Guide](docs/usage.md)**

---

> Built for everyday workflows now, with room to grow into real tooling later. If it clicks for you, a star means a lot. ⭐

[MIT License](LICENSE)
