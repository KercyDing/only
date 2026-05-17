# Only

[![crates.io](https://img.shields.io/crates/v/only.svg)](https://crates.io/crates/only)
[![license](https://img.shields.io/crates/l/only.svg)](LICENSE)

**One `Onlyfile`. Same behavior everywhere.**

`only` is a small task runner for projects that need to work the same on macOS, Linux, and Windows.

Write this once:

```Onlyfile
# Start small.
serve(port="3000", host="127.0.0.1"):
    echo "Serving on {{host}}:{{port}}"

check():
    cargo check

test():
    cargo test

ci() & check & test:
    echo "done"
```

Run it like this:

```shell
only
only serve
only serve 8080
only --set host=0.0.0.0 serve 8080
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

```shell
cargo install only
```

From GitHub:

```shell
cargo install --git https://github.com/KercyDing/only only
```

## Learn more

See a complete example: [examples/Onlyfile](examples/Onlyfile).

See the full guide: [docs/usage.md](docs/usage.md).

## LICENSE

[MIT](LICENSE)
