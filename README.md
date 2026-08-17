# Only

[![crates.io](https://img.shields.io/crates/v/only.svg)](https://crates.io/crates/only)
[![license](https://img.shields.io/crates/l/only.svg)](LICENSE)

**One `Onlyfile`. Same behavior everywhere.**

`only` is a small cross-platform task runner for macOS, Linux, and Windows.

Dependencies, parallelism, environment-specific variants, namespaces, and common shell behavior all stay in one readable `Onlyfile`.

## Example

```Onlyfile
# Start development.
dev(port="3000"):
    pnpm dev --port {{ port }}

# Check the project.
check():
    cargo check

# Prefer nextest when available.
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test

# Run checks and tests in parallel.
ci() & (check, test):
    echo "CI complete"
```

Run it:

```shell
only
only dev
only dev 8080
only ci
```

Or inspect the execution plan first:

```shell
only --dry-run ci
```

```text
Dry run: ci()
├─ stage 1 (parallel)
│  ├─ check
│  └─ test
└─ stage 2
   └─ ci
```

Dependencies, parallel execution, parameters, and environment-specific task variants stay in the task definition instead of shell control flow.

For a larger frontend + Rust workflow, see the [usage guide](docs/usage.md).

## Features

- **Cross-platform by default** — common commands behave consistently across macOS, Linux, and Windows.
- **Task dependencies** — describe ordering directly in the task signature.
- **Parallel stages** — group independent dependencies with `(a, b)`.
- **Guards** — select implementations with `@has`, `@os`, `@arch`, and `@env`.
- **Namespaces** — organize larger projects with `[front]`, `[back]`, `[version]`, etc.
- **Parameters** — use function-style task signatures and `{{ value }}` interpolation.
- **Private helpers** — tasks beginning with `_` stay out of the normal task list.
- **Dry run** — inspect the resolved execution plan before running it.
- **Shell escape hatch** — request `bash`, `sh`, `pwsh`, or PowerShell when needed.

## Why not `just` or Task?

| Tool | Good for | Tradeoff |
| --- | --- | --- |
| [`just`](https://github.com/casey/just) | shell-oriented command recipes | workflow behavior still depends heavily on the host shell |
| [`Task`](https://taskfile.dev/) | YAML-based automation | more configuration for projects that want a compact task language |
| `only` | structured project workflows | introduces a small dedicated task syntax |

`only` aims to stay close to the simplicity of a command runner while making workflow structure explicit.

## Install

### macOS / Linux

```shell
curl -fsSL https://raw.githubusercontent.com/KercyDing/only/master/install.sh | sh
```

### Windows PowerShell

```powershell
irm https://raw.githubusercontent.com/KercyDing/only/master/install.ps1 | iex
```

### Cargo

```shell
cargo install only
```

### Arch Linux

```shell
paru -S only-bin
```

For the latest Git version:

```shell
paru -S only-git
```

### GitHub

```shell
cargo install --git https://github.com/KercyDing/only only
```

## Update

```shell
only --upgrade
```

or:

```shell
only --update
```

## Editor support

The [Onlyfile VS Code extension](https://marketplace.visualstudio.com/items?itemName=kercyding.onlyfile) provides syntax highlighting and `only-lsp` integration for diagnostics, hover, document symbols, and folding.

## Learn more

- [Complete example](examples/Onlyfile)
- [Usage guide](docs/usage.md)

## License

[MIT](LICENSE)
