<div align="center">

# Only

**⚡️A small cross-platform task runner.⚡️**

Keep project workflows in one readable `Onlyfile`.

[![Crates.io](https://img.shields.io/crates/v/only.svg)](https://crates.io/crates/only)
[![CI](https://github.com/KercyDing/only/actions/workflows/check.yml/badge.svg)](https://github.com/KercyDing/only/actions/workflows/check.yml)
[![License](https://img.shields.io/crates/l/only.svg)](LICENSE)

<kbd>[Usage guide](docs/usage.md)</kbd> <kbd>[Complete example](examples/Onlyfile)</kbd> <kbd>[VSCode extension](https://marketplace.visualstudio.com/items?itemName=kercyding.onlyfile)</kbd>

</div>

## Example

```Onlyfile
!version 0.2

# Build the Rust app
build(profile="dev"):
    cargo build --profile {{profile}}

# Run tests in parallel
ci() & (back.test, front.test):
    echo "CI complete!"

# Backend
[back]

# Test the backend
// Prefer nextest when available.
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test

# Frontend
[front]

# Test the frontend
test():
    pnpm test
```

Run it:

```shell
only
only build
only build release
only ci
```

Or inspect the execution plan first:

```shell
only --dry-run ci
```

```text
Dry run: ci()
├─ stage 1 (parallel)
│  ├─ back.test (1 command)
│  └─ front.test (1 command)
└─ stage 2
   └─ ci (1 command)
```

Dependencies, parallel execution, parameters, and environment-specific task variants stay in the task definition instead of shell control flow.

## Features

- **Cross-platform by default** — common commands behave consistently across macOS, Linux, and Windows.
- **Task dependencies** — describe ordering directly in the task signature.
- **Parallel stages** — group independent dependencies with `(a, b)`.
- **Guards** — select implementations with `@has`, `@os`, `@arch`, and `@env`.
- **Namespaces** — organize larger projects with `[front]`, `[back]`, `[version]`, etc.
- **Parameters** — use function-style task signatures and `{{ value }}` interpolation.
- **Command blocks** — run consecutive `|` lines in one cross-platform shell process.
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

## License

[MIT](LICENSE)
