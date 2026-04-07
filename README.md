# Only

A minimalist, deterministic task runner.

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

## Notes

- `only` searches for `Onlyfile` or `onlyfile` in the current directory and parent directories.
- `only` with no arguments lists available tasks when an `Onlyfile` is found.
- `only <namespace>` prints help for that namespace and its child tasks.
- Tasks run from the discovered `Onlyfile` directory, not necessarily from the shell's current directory.
- Task parameters are positional. `--set NAME=VALUE` can override task parameters from the CLI.
- With `!verbose true`, `only` prints a task banner, numbered command steps, and step-aware failure messages.
- `{{name}}` interpolation is plain text substitution. It does not perform shell escaping, so task authors must quote or escape parameters when needed.

## Docs

- Usage guide: [docs/usage.md](docs/usage.md)
