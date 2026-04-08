# Only

**Write tasks once. Run them everywhere — exactly the same.**

Only is a modern, cross-platform, deterministic task runner designed for developers who hate shell compatibility issues.

No Git Bash.  
No `if os()` hacks.  
No `platforms:` boilerplate.  

Just one clean `Onlyfile` that works identically on **macOS, Linux, and Windows** — powered by `deno_task_shell` by default.

---

## Quick Start

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
    cargo install --path . --force

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

## Why developers choose Only over Just and Taskfile

- **True cross-platform consistency** — default `deno_task_shell` means your commands behave the same everywhere, no extra configuration needed
- **Cleaner, more modern syntax** — function-style signatures with parameters and defaults
- **Simple dependency chaining** — `&` keeps common task pipelines readable
- **Better out-of-the-box experience** — dynamic help, colored output, clean task listing

**Just** is powerful but often requires manual shell setup on Windows.  
**Taskfile** is solid but uses heavier YAML and still needs platform rules.

**Only is ideal for personal projects, small-to-medium repositories, and developers who want simplicity and reliability** without fighting with configuration.

---

## Installation

```shell
cargo install --git https://github.com/KercyDing/only
```

---

## Docs

Full syntax guide and examples → **[docs/usage.md](docs/usage.md)**

---

> Under active development and already powering real workflows.  
> Star if you like where this is going ✨

[MIT License](LICENSE)
