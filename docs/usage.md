# Usage

`only` is a deterministic task runner driven by an `Onlyfile`.

## File Discovery

By default, `only` looks for `Onlyfile` or `onlyfile` in:

- the current directory
- each parent directory until the filesystem root

You can also point to a file explicitly:

```bash
only -f ./Onlyfile
```

Print the resolved path without running a task:

```bash
only --path
```

When a task runs, `only` uses the discovered `Onlyfile` directory as the task working directory.

For example, if `only` finds `/repo/Onlyfile` while you are currently in `/repo/src`, task commands still run from `/repo`.

## CLI Basics

Show program help:

```bash
only --help
```

List available tasks in the discovered `Onlyfile`:

```bash
only
```

Run a global task:

```bash
only test
```

Show help for a namespace:

```bash
only dev
```

Run a task inside a namespace:

```bash
only dev smoke
only dev smoke "hello world"
```

Override task parameters with global options:

```bash
only run hello
only --set task=hello run
only --set name="hello world" dev smoke
```

## Onlyfile Structure

An `Onlyfile` contains:

- optional top-level directives
- global tasks
- optional namespace sections

Global tasks should be defined before the first namespace header. After `only` enters a namespace section, following top-level tasks belong to that namespace until the next namespace header or end of file.

Example:

```text
!verbose true

% Format the project.
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

## Syntax

### Comments

Lines starting with `#` are ignored.

```text
# This is a comment.
```

### Doc Comments

Lines starting with `%` document the next task. These descriptions are shown in task listings and help output.

```text
% Run tests.
test():
    cargo test
```

### Directives

Directives must appear before any task or namespace.

Current supported directive:

- `!verbose true`
- `!verbose false`
- `!shell deno`
- `!shell sh`
- `!shell bash`
- `!shell powershell`
- `!shell pwsh`

`only` uses `deno_task_shell` by default. This gives cross-platform shell behavior without requiring `/bin/sh`.

Use `!shell ...` when you want to force a specific backend:

```text
!shell bash
```

Notes:

- `!shell deno` uses the default cross-platform backend.
- `!shell sh` and `!shell bash` require those executables to exist on the host system.
- `!shell powershell` and `!shell pwsh` require the corresponding PowerShell executable to exist on the host system.

When `!verbose true` is enabled, `only` prints each task and command before executing it.

If the task working directory differs from the shell's current directory, the task banner shows the effective directory:

```text
[task] check (at /path/to/project)
```

Commands are shown with step numbers:

```text
[task] check
  [1/3] cargo check
  [2/3] cargo fmt --all --check
  [3/3] cargo clippy --workspace -- -D warnings
```

If a command fails, the error points to the failing step:

```text
Error: task 'check' failed at step [2/3] while running `cargo fmt --all --check` with exit code ...
```

```text
!verbose true
```

### Tasks

A task definition has a signature followed by indented command lines.

```text
build():
    cargo build
```

Task names are flat identifiers. There is no special `default()` task behavior.

### Parameters

Task parameters are declared inside `()`.

```text
serve(port="3000", host="127.0.0.1"):
    echo "{{host}}:{{port}}"
```

Rules:

- Parameters are strings.
- Parameters without a default are required.
- Parameters with defaults are optional.
- CLI invocation uses positional arguments in declaration order.

Examples:

```bash
only serve
only serve 8080
only serve 8080 0.0.0.0
```

You can also override by name:

```bash
only --set port=8080 serve
only --set host=0.0.0.0 --set port=8080 serve
```

### Interpolation

Use `{{name}}` inside command bodies to inject parameter values.

```text
smoke(name="hello"):
    echo "{{name}}"
```

Important:

- Interpolation is plain text substitution.
- `only` does not do shell escaping.
- If a value may contain spaces, quotes, `$`, `*`, or other shell-sensitive characters, the task author must quote or escape it in the command.

Safer example:

```text
smoke(name="hello world"):
    echo "{{name}}"
```

## Namespaces

Namespaces group tasks under a flat section header.

```text
[dev]
% Run formatter and tests.
workflow() & fmt & test:
    echo "dev complete"

% Run a smoke command.
smoke(name="hello"):
    echo "{{name}}"
```

Run them like this:

```bash
only dev
only dev workflow
only dev smoke
only dev smoke "hello world"
```

Notes:

- `only dev` shows namespace help.
- `only dev smoke` runs the `smoke` task.
- Namespaces are flat labels, not nested command trees.
- A namespace name can contain `.` for visual grouping, such as `[deploy.prod]`.

## Dependencies

Use `&` in the task signature to declare serial dependencies.

```text
ci() & fmt & test:
    echo "CI complete"
```

Dependencies are resolved before the task itself runs.

Inside a namespace:

- `& test` first looks for `test` in the same namespace
- if not found, it falls back to a global task with that name

Cross-namespace dependencies use `namespace.task`:

```text
[backend]
release():
    cargo build --release

[docker]
build() & backend.release:
    docker build -t app:latest .
```

## Guards

Use `? @probe("value")` to select a task only when a condition matches.

```text
build() ? @os("linux"):
    cargo build

build():
    echo "fallback build"
```

Current probes:

- `@os("linux")`
- `@arch("x86_64")`
- `@env("CI")`
- `@cmd("cargo")`

Notes:

- If multiple tasks share the same name, guard resolution is top-down.
- An unguarded task acts as the fallback.

## Error Cases

Typical failures include:

- no `Onlyfile` found in the current directory or any parent
- missing required parameter
- unknown parameter in `--set`
- undefined dependency
- invalid syntax in the `Onlyfile`
- command exits with a non-zero status
- verbose command failures include the failing step number

When no `Onlyfile` is found:

```bash
only
```

prints an error and suggests:

```bash
only --help
```

## Practical Example

```text
!verbose true

% Format the Rust codebase.
fmt():
    cargo fmt --all

% Run cargo check.
check():
    cargo check
    cargo fmt --all --check
    cargo clippy --workspace -- -D warnings

% Run the full test suite.
test():
    cargo test

% Run formatter, checks, and tests.
ci() & fmt & check & test:
    echo "CI complete"

[dev]
% Developer workflow.
workflow() & fmt & test:
    echo "dev complete"

% Run a namespaced smoke command.
smoke(name="It's a smoke command"):
    echo "{{name}}"
```

Usage:

```bash
only
only ci
only dev
only dev workflow
only dev smoke
only dev smoke "custom message"
```
