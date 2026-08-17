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
!version 0.3
!var cargo_flags = "--workspace"

# Build the Rust app
build(profile = "dev"):
    cargo build {{cargo_flags}} --profile {{profile}}

# Run tests in parallel
ci() & (back.test, front.test):
    echo "CI complete!"

# Backend
[back] {
    # Test the backend.
    // Prefer nextest when available.
    test() ? @env("CI") ? @has("cargo-nextest"):
        cargo nextest run

    test():
        cargo test
}

# Frontend
[front] {
    # Test the frontend.
    test():
        pnpm test
}
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

Format the Onlyfile with:

```shell
only --fmt
```

Dependencies, parallel execution, parameters, and environment-specific task variants stay in the task definition instead of shell control flow.

## Features

- **Structured tasks** — parameters, dependencies, namespaces, and private helpers.
- **Predictable execution** — ordered stages, parallel groups, and deduplicated dependencies.
- **Environment-aware variants** — guards for the OS, architecture, environment, and installed commands.
- **Cross-platform commands** — a built-in shell, command blocks, and optional host shells.
- **Consistent files** — global string values, multiline headers, and a zero-config formatter.
- **Inspectable workflows** — validation, dynamic help, and dry-run execution plans.

## How it differs

All three support dependencies and cross-platform workflows. The main difference is how they express them.

| Tool | Configuration style | Emphasis |
| --- | --- | --- |
| [`just`](https://github.com/casey/just) | Make-inspired recipes in a `justfile` | mature, shell-oriented command recipes |
| [`Task`](https://taskfile.dev/) | declarative YAML in `Taskfile.yml` | broad task configuration without a custom syntax |
| `only` | function-style tasks in an `Onlyfile` | explicit stages, guarded variants, and namespaces in compact syntax |

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
