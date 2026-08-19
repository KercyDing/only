# Usage

`only` reads tasks from an `Onlyfile` and runs them on macOS, Linux, and Windows.

This guide shows you the language step by step. You can start with one task, then add parameters, dependencies, parallel work, guards, shells, and groups as your project grows.

For a complete workflow, see the [example Onlyfile](../examples/Onlyfile).

## Contents

- [Language version](#language-version)
- [1. Create your first Onlyfile](#1-create-your-first-onlyfile)
- [2. Document your tasks](#2-document-your-tasks)
- [3. Add parameters](#3-add-parameters)
  - [Required parameters](#required-parameters)
  - [Slice parameters](#slice-parameters)
  - [Named overrides](#named-overrides)
  - [Literal interpolation braces](#literal-interpolation-braces)
- [4. Add dependencies](#4-add-dependencies)
- [5. Run independent tasks in parallel](#5-run-independent-tasks-in-parallel)
  - [Inspect the execution plan](#inspect-the-execution-plan)
- [6. Hide internal helper tasks](#6-hide-internal-helper-tasks)
- [7. Select task versions with guards](#7-select-task-versions-with-guards)
- [8. Choose a shell when needed](#8-choose-a-shell-when-needed)
  - [Run several lines in one shell](#run-several-lines-in-one-shell)
  - [Set a shell for the whole file](#set-a-shell-for-the-whole-file)
- [9. Organize larger projects with groups](#9-organize-larger-projects-with-groups)
- [10. Put the pieces together](#10-put-the-pieces-together)
- [Troubleshooting](#troubleshooting)
- [Quick reference](#quick-reference)

---

## Language version

Use this line to tell `only` the oldest language version your file needs:

```Onlyfile
!version MAJOR.MINOR
```

For example, `!version 0.1` has a major number and a minor number.

This line is optional. Without it, `only` reads the file without checking its language version first.

When you use `!version`, place it before all other declarations. Blank lines and comments may come before it.

---

## 1. Create your first Onlyfile

Create a file named `Onlyfile` in your project root:

```Onlyfile
hello():
    echo "hello from only"
```

Run the task:

```bash
only hello
```

Run `only` without a task to see the available tasks:

```bash
only
```

`only` looks for `Onlyfile` or `onlyfile` in the current directory and then walks upward.

To see which file was found:

```bash
only --where
```

You can also select a file yourself:

```bash
only ci -p ./examples/Onlyfile --dry-run
```

## 2. Document your tasks

Use `#` for normal comments:

```Onlyfile
# This is only a source comment.
check():
    cargo check
```

Use `[help]` to give the next task or group a short description:

```Onlyfile
[help] Check the project
[desc] Check all build targets.
[pass] Check passed.
[fail] Check failed.
check():
    cargo check
```

`[help]` appears in the task list and help page. Add one or more `[desc]` lines when you need more detail. `[desc]` can be used on its own.

`[pass]` is printed after the task succeeds. `[fail]` is printed after it fails. You can use either one without `[help]`.

Keep these lines directly above the task. A blank line separates them from the task.

---

## 3. Add parameters

Add parameters inside the parentheses after a task name:

```Onlyfile
build(profile="dev"):
    cargo build --profile {{profile}}
```

Run it with the default:

```bash
only build
```

Or pass another value:

```bash
only build release
```

`{{profile}}` inserts the chosen value into the command.

A parameter with a default is optional.

### Required parameters

Leave out the default to make a parameter required:

```Onlyfile
greet(name):
    echo "Hello, {{name}}"
```

Pass the value positionally:

```bash
only greet Ada
```

If you run:

```bash
only greet
```

`only` reports the missing parameter before running any command.

### Slice parameters

Use `..` to collect all remaining values:

```Onlyfile
run(args..):
    cargo run {{args}}
```

Then:

```bash
only run --release --bin demo
```

binds `args..` to:

```text
--release --bin demo
```

A slice can follow fixed parameters:

```Onlyfile
tool(name, args..):
    {{name}} {{args}}
```

A slice parameter:

- accepts zero or more values;
- must be the final parameter;
- cannot have a default value.

### Named overrides

Use `-s` or `--set` when you want to set a parameter by name:

```Onlyfile
serve(port="3000", host="127.0.0.1"):
    echo "{{host}}:{{port}}"
```

Override only the host:

```bash
only serve -s host=0.0.0.0
```

Override more than one value:

```bash
only serve -s host=0.0.0.0 -s port=8080
```

Place `-s` after the task arguments.

Parameter values are chosen in this order:

1. `-s NAME=VALUE` / `--set NAME=VALUE`
2. positional arguments
3. defaults from the task signature

### Literal interpolation braces

Use `\{{` and `\}}` when you need literal braces:

```Onlyfile
show(value="demo"):
    echo "{{value}} \{{value\}}"
```

This prints:

```text
demo {{value}}
```

---

## 4. Add dependencies

Use `&` when one task must run before another.

```Onlyfile
check():
    cargo check

test():
    cargo test

ci() & check & test:
    echo "CI complete"
```

Run:

```bash
only ci
```

The order is:

```text
check
  ↓
test
  ↓
 ci
```

Each `&` adds another step.

Dependencies are tasks, not copies of their commands. If several tasks need the same dependency, `only` runs it once.

You can pass values to a dependency:

```Onlyfile
package() & build("release"):
    echo "Package complete"
```

A dependency without values uses its defaults. If it has a required parameter, pass the value as shown above.

---

## 5. Run independent tasks in parallel

Put independent dependencies inside parentheses:

```Onlyfile
ci() & (back.test, front.test):
    echo "CI complete"
```

Here, `back.test` and `front.test` may run at the same time.

Compare these two forms:

```Onlyfile
ci() & check & test:
```

```text
check → test → ci
```

and:

```Onlyfile
ci() & (check, test):
```

```text
check ─┐
       ├→ ci
test ──┘
```

Use sequential stages when tasks depend on each other.

Use a parallel group when the tasks are independent.

### Inspect the execution plan

You do not need to guess how a workflow will run.

Use dry-run:

```bash
only ci --dry-run
```

To include rendered commands:

```bash
only ci --dry-run --full
```

For larger workflows, dry-run shows the final order after `only` chooses the matching task variants.

---

## 6. Hide internal helper tasks

Prefix a task name with `_` to make it a helper:

```Onlyfile
_prepare():
    cargo check

ci() & _prepare:
    cargo test
```

Helpers:

- can be used as dependencies;
- do not appear in normal task listings;
- cannot be run directly.

Use helpers for setup steps that users do not need to run by hand.

---

## 7. Select task versions with guards

A task can have more than one way to run.

Use `?` with a guard to choose the first version that matches your system:

```Onlyfile
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test
```

If `cargo-nextest` is available as a command, the first variant is used.

Otherwise, the unguarded task is the fallback.

You can also check the operating system:

```Onlyfile
package() ? @os("windows"):
    echo "Package for Windows"

package():
    echo "Package for Unix"
```

Available checks include:

| Probe | Matches when |
| --- | --- |
| `@os("macos")` | the current OS is macOS |
| `@os("linux")` | the current OS is Linux |
| `@os("windows")` | the current OS is Windows |
| `@arch("x86_64")` | the current architecture is x86-64 |
| `@arch("aarch64")` | the current architecture is AArch64 |
| `@env("CI")` | the environment variable exists |
| `@has("cargo")` | `cargo` is available as a command |

Variant selection follows three rules:

1. the first matching guarded variant wins;
2. if no guard matches, the unguarded variant is used;
3. if nothing matches and there is no fallback, the task is unavailable.

The first variant provides the default `[help]`, `[desc]`, `[pass]`, and `[fail]` text. Later variants keep any fields they leave out. When a later variant writes a field, it replaces the whole field.

Guards let you choose a task version without putting system checks in shell scripts.

---

## 8. Choose a shell when needed

By default, commands use the built-in cross-platform `deno` shell.

For common tasks, you normally do not need to choose a shell:

```Onlyfile
check():
    cargo check
```

Choose a specific shell only when your command needs one.

Require Bash:

```Onlyfile
build() shell=bash:
    ./scripts/build.sh
```

Use `shell~=` when `only` may use a backup shell:

```Onlyfile
build() shell~=bash:
    ./scripts/build.sh
```

Known fallbacks are:

```text
pwsh → powershell
bash → sh
```

You can combine shells with guards:

```Onlyfile
show-user() ? @os("windows") shell~=pwsh:
    Write-Output $env:USERNAME
```

Supported shells are:

| Shell | Behavior |
| --- | --- |
| `deno` | built-in cross-platform shell |
| `bash` | runs `bash -c` |
| `sh` | runs `sh -c` |
| `pwsh` | runs PowerShell 7+ |
| `powershell` | runs Windows PowerShell |

### Run several lines in one shell

Each normal command line starts a new shell.

Use `|` when several lines need to share the same shell process:

```Onlyfile
version() shell=bash:
    | version=$(git describe --tags --always)
    | echo "$version"
```

Both lines run in the same shell, so the second line can use the variable created by the first.

You can mix command blocks with normal commands:

```Onlyfile
paths() shell=bash:
    | cd crates/cli
    | pwd
    pwd
```

The first `pwd` prints a path ending in `crates/cli`. The final `pwd` starts a new shell and prints the project root.

Use a bare `|` when you need a blank line inside a command block.

### Set a shell for the whole file

Use `!shell` if most tasks need the same shell:

```Onlyfile
!shell bash

hello():
    echo "hello"
```

Shell selection follows this order:

1. task-level `shell=` or `shell~=`
2. file-level `!shell`
3. built-in `deno`

Available file-level directives are:

| Directive | Purpose |
| --- | --- |
| `!version MAJOR.MINOR` | tell `only` the oldest language version your file needs |
| `!shell NAME` | set the default shell |

---

## 9. Organize larger projects with groups

Use groups when several parts of your project have similar tasks.

```Onlyfile
group front {

    check():
        pnpm lint

    test():
        pnpm test
}

group back {

    check():
        cargo check

    test():
        cargo test
}
```

Run a grouped task by placing the group before the task:

```bash
only front check
only front test
only back check
only back test
```

Run only the group name to see its tasks:

```bash
only front
```

You can also refer to grouped tasks from dependencies:

```Onlyfile
ci() & (front.test, back.test):
    echo "CI complete"
```

This lets you keep short task names while still showing which part of the project they belong to.

---

## 10. Put the pieces together

You can combine tasks, parameters, guards, groups, and parallel dependencies without turning the file into a large script.

Here is a small frontend + Rust example:

```Onlyfile
build(profile="dev"):
    cargo build --profile {{profile}}

ci() & (back.test, front.test):
    echo "CI complete"

group back {

    test() ? @has("cargo-nextest"):
        cargo nextest run

    test():
        cargo test
}

group front {

    test():
        pnpm test
}
```

Now you can run:

```bash
only build
only build release
only back test
only front test
only ci
```

Inspect the workflow before running it:

```bash
only ci --dry-run --full
```

The important idea is simple:

> You say which tasks must run first. `only` works out the order.

---

# Troubleshooting

## `only` is using the wrong file

Check the discovered file:

```bash
only --where
```

Or select one explicitly:

```bash
only ci -p ./Onlyfile
```

---

## A task does not appear in `only`

Check whether:

- the task starts with `_`;
- the task belongs to a group;
- the group contains only hidden helper tasks;
- the `Onlyfile` contains an error.

For a grouped task, use:

```bash
only <group> <task>
```

---

## `-s` / `--set` does not work

Make sure:

1. the parameter exists;
2. the task uses the expected parameter;
3. `-s` appears before the task path.

For example:

```bash
only serve -s port=8080
```

---

## A positional argument sets the wrong parameter

Positional values follow declaration order.

Given:

```Onlyfile
serve(port="3000", host="127.0.0.1"):
    echo "{{host}}:{{port}}"
```

this:

```bash
only serve 8080
```

sets `port`.

To change only `host`, use:

```bash
only serve -s host=0.0.0.0
```

---

## A guarded task is unavailable

This means no guarded variant matched and no fallback exists.

Add a fallback when appropriate:

```Onlyfile
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test
```

---

## A shell is missing

Prefer the built-in shell when possible.

If you need a specific shell, you can:

- install the shell;
- choose another supported shell;
- use `shell~=` when a compatible fallback is acceptable.

Example:

```Onlyfile
build() shell~=bash:
    ./scripts/build.sh
```

Supported names are:

```text
deno
bash
sh
pwsh
powershell
```

---

## Interpolation fails

Check that:

- `{{name}}` matches a declared parameter;
- every unescaped `{{` has a closing `}}`;
- literal braces use `\{{` and `\}}`.

---

# Quick reference

## CLI

| Command | Purpose |
| --- | --- |
| `only` | list available tasks |
| `only <task>` | run a root task |
| `only <group> <task>` | run a grouped task |
| `only --help` | show help |
| `only <task> --help` | show task help and parameters |
| `only --where` | print the discovered `Onlyfile` path |
| `only <task> -p <path>` / `only <task> --path <path>` | use a specific file |
| `only <task> -s name=value` | override a parameter |
| `only <task> --dry-run` | show the execution plan |
| `only <task> --dry-run --full` | show the plan and rendered commands |
| `only <task> -q` / `only <task> --quiet` | hide progress lines but keep command output |
| `only --fmt` | format the `Onlyfile` |
| `only --check` | check `Onlyfile` formatting |
| `only --upgrade` | update `only` |

## Syntax

| Syntax | Meaning |
| --- | --- |
| `// text` | ordinary comment |
| `# text` | ordinary comment |
| `!version MAJOR.MINOR` | tell `only` the oldest language version your file needs |
| `!shell bash` | set the file-level shell |
| `task():` | define a task |
| `task(name):` | required parameter |
| `task(name="value"):` | parameter with a default |
| `task(args..):` | slice parameter |
| `{{name}}` | interpolate a parameter |
| `_task():` | define a helper task |
| `task() & a & b:` | ordered dependency stages |
| `task() & (a, b):` | parallel dependency stage |
| `task() ? @has("cmd"):` | guarded task variant |
| `task() shell=bash:` | require an exact shell |
| `task() shell~=bash:` | prefer a shell with fallback |
| `\| command` | continue a command block in one shell process |
| `group name { ... }` | group related tasks |
