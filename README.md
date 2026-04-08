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

% Format the codebase.
fmt():
    cargo fmt --all

% Run tests.
test():
    cargo test

[dev]
% Build in development mode.
dev():
    cargo build

% Build in release mode.
rel():
    cargo build --release
```

Then run:

```shell
only                # list all tasks
only fmt
only test
only dev smoke "world"
```

### Advanced Example

```Onlyfile
!verbose true

% Build the project
# Windows uses PowerShell, others use default shell
build() ? @os("windows") shell?=pwsh:
    cargo build --release

build():
    cargo build --release

% Clean build artifacts
# Run with platform-native commands
clean() ? @os("windows") shell?=pwsh:
    Remove-Item -Recurse -Force target, dist -ErrorAction SilentlyContinue

clean() ? @os("linux") shell=sh:
    rm -rf target dist

clean() ? @os("macos") shell?=bash:
    rm -rf target dist

clean():
    rm -rf target dist

% Run checks only if cargo is available
check() ? @has("cargo"):
    cargo check
    cargo fmt --all --check
    cargo clippy --workspace -- -D warnings

check():
    echo "cargo not found, skipping checks"

% Full CI pipeline
ci() & fmt & check & test:
    echo "✅ CI completed successfully"
```

---

## Why developers choose Only over Just and Taskfile

- **True cross-platform consistency** — default `deno_task_shell` means your commands behave the same everywhere, no extra configuration needed
- **Cleaner, more modern syntax** — function-style signatures with parameters and defaults
- **More intuitive dependencies** — `&` for serial, `|` for parallel, with parentheses for complex flows
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
