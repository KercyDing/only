<div align="center">

# Only

**A task language for explicit, cross-platform workflows.**

[![Crates.io](https://img.shields.io/crates/v/only.svg)](https://crates.io/crates/only)
[![CI](https://github.com/KercyDing/only/actions/workflows/check.yml/badge.svg)](https://github.com/KercyDing/only/actions/workflows/check.yml)
[![License](https://img.shields.io/crates/l/only.svg)](LICENSE)

<kbd>[Usage guide](docs/usage.md)</kbd> <kbd>[Complete example](examples/Onlyfile)</kbd> <kbd>[VS Code extension](https://marketplace.visualstudio.com/items?itemName=kercyding.onlyfile)</kbd>

</div>

## Quick start

An `Onlyfile` describes tasks and their execution stages:

```Onlyfile
!version 0.4
!var pnpm_dir = "web"

# Internal setup task.
[fail] Preparation failed
_prepare():
    | cd {{pnpm_dir}}
    | pnpm install
    cargo fetch

[help] Run CI
[pass] CI complete
ci()
    & _prepare
    & (front.check, back.check)
    & (front.test, back.test)
:

[help] Frontend checks
[desc] The pnpm dir is {{pnpm_dir}}/
group front {

    check():
        pnpm --dir {{pnpm_dir}} lint

    test():
        pnpm --dir {{pnpm_dir}} test
}

[help] Backend checks
group back {

    check():
        cargo check

    test() ? @has("cargo-nextest"):
        cargo nextest run

    test():
        cargo test
}
```

`&` advances to the next dependency stage; tasks inside `( ... )` run in parallel.

Run `only` to list tasks, or run one directly:

```shell
only
only ci
```

Inspect the resolved workflow before running it:

```shell
only ci --dry-run --full
```

```text
Dry run: ci()
├─ stage 1
│  └─ _prepare
│     ├─ block (deno)
│     │  ├─ cd web
│     │  └─ pnpm install
│     └─ cargo fetch
├─ stage 2 (parallel)
│  ├─ front.check
│  │  └─ pnpm --dir web lint
│  └─ back.check
│     └─ cargo check
├─ stage 3 (parallel)
│  ├─ front.test
│  │  └─ pnpm --dir web test
│  └─ back.test
│     └─ cargo nextest run
└─ stage 4
   └─ ci
```

Format the file with:

```shell
only --fmt
```

The workflow structure stays in the task definition instead of being hidden in shell control flow. See the [usage guide](docs/usage.md) for parameters, guards, shells, command blocks, and groups.

## Why Only

* **Explicit stages** — `&` expresses order and `( ... )` expresses parallelism.
* **Structured tasks** — parameters, guards, groups, helpers, and metadata are part of the language.
* **Cross-platform commands** — use the built-in shell by default, or select a host shell when needed.
* **Inspectable workflows** — the formatter, diagnostics, editor tooling, and `--dry-run --full` understand the same task graph.

## Compared with just and Task

`just`, Task, and Only can all run project commands. Only focuses on making execution stages and parallel groups explicit in the task definition.

| Tool                                    | Model                      | Best fit                       |
| --------------------------------------- | -------------------------- | ------------------------------ |
| [`just`](https://github.com/casey/just) | recipes and dependencies   | command recipes and scripts    |
| [`Task`](https://taskfile.dev/)         | YAML task definitions      | declarative task configuration |
| **Only**                                | tasks and execution stages | explicit workflow structure    |

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
