# Only

**Write tasks once. Run them everywhere — exactly the same.**

Only is a modern, cross-platform, deterministic task runner built for developers who are tired of shell compatibility drama.

No Git Bash on Windows.  
No `if os()` hacks.  
No `platforms:` boilerplate.  

Just one clean `Onlyfile` that works identically on **macOS, Linux, and Windows** — powered by `deno_task_shell` by default.

---

## Quick Start

Create an `Onlyfile` in your project root:

```text
!verbose true

% Run formatter.
fmt():
    cargo fmt --all

% Run tests.
test():
    cargo test

[dev]
% Run a smoke command.
smoke(name="hello"):
    echo "{{name}}"
```

Then run:

```bash
only                # list all available tasks
only fmt
only test
only dev            # show namespace subcommands
only dev smoke "hello world"
```

---

## Installation

```bash
# Cargo (recommended)
cargo install --git https://github.com/KercyDing/only

# Or build from source
git clone https://github.com/KercyDing/only
cd only
cargo install --path .
```

---

## Why developers are switching

- **True cross-platform consistency** — default `deno_task_shell` means your commands behave the same everywhere  
- **Modern, readable syntax** — function-style signatures with parameters and defaults  
- **Natural namespace subcommands** — `only dev smoke` feels like a real CLI  
- **Beautiful out-of-the-box experience** — colored output, dynamic `--help`, clean task listing, progress indicators  
- **Deterministic by design** — same task, same result, every time

---

## Docs

Full syntax guide, advanced features, and examples → **[docs/usage.md](docs/usage.md)**

---

> Project is under active development and already powering real workflows.  
> Star the repo if you like where this is going ✨
