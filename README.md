# Only

A cross-platform, deterministic task runner for modern projects.

Write tasks once in an `Onlyfile`, then run them the same way across macOS, Linux, and Windows.

> Project is currently under active development.

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
only
only --help
only fmt
only dev
only dev smoke "hello world"
```

## Docs

- Usage guide: [docs/usage.md](docs/usage.md)
