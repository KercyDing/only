# Only

[![crates.io](https://img.shields.io/crates/v/only.svg)](https://crates.io/crates/only)
[![license](https://img.shields.io/crates/l/only.svg)](LICENSE)

**One `Onlyfile`. Same behavior everywhere.**

`only` is a small task runner for projects that need to work the same on macOS, Linux, and Windows.

Write this once:

```Onlyfile
// Start small and keep common commands close to the project.
# Start the development server.
serve(port="3000", host="127.0.0.1"):
    echo "Serving on {{host}}:{{port}}"

# Check the project.
check():
    cargo check

# Run tests.
test():
    cargo test

# Run the local CI workflow.
ci() & check & test:
    echo "done"
```

Run it like this:

```shell
only
only serve
only serve 8080
only -s host=0.0.0.0 serve 8080
only ci
```

That is the core idea: a readable task file, parameters when you need them, dependencies when tasks grow, and no per-platform shell surprises.

## Why not just/taskfile?

| Tool | Good for | Tradeoff |
| --- | --- | --- |
| `just` | simple command aliases | still depends a lot on the user's shell |
| `taskfile` | bigger YAML workflows | more config than many projects need |
| `only` | small tasks that may grow into tooling | a real task syntax instead of plain shell or YAML |

## Install

From the latest GitHub release:

```shell
curl -fsSL https://raw.githubusercontent.com/KercyDing/only/master/install.sh | sh
```

On Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/KercyDing/only/master/install.ps1 | iex
```

From crates.io:

```shell
cargo install only
```

On Arch Linux:

```shell
# Stable release
paru -S only-bin

# Git build
paru -S only-git
```

From GitHub:

```shell
cargo install --git https://github.com/KercyDing/only only
```

## Update

```shell
only --upgrade
# or: only --update
```

## VSCode extension support

The [Onlyfile extension](https://marketplace.visualstudio.com/items?itemName=kercyding.onlyfile) provides syntax highlighting and `only-lsp` integration for diagnostics, hover, document symbols, and folding.

## Learn more

See a complete example: [examples/Onlyfile](examples/Onlyfile).

See the full guide: [docs/usage.md](docs/usage.md).

## LICENSE

[MIT](LICENSE)
