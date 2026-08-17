# Usage

`only` is a cross-platform task runner driven by an `Onlyfile`.

This guide teaches the language step by step. You can start with a single task, then add parameters, dependencies, parallel work, guards, shells, and namespaces as your project grows.

For a complete workflow, see the [example Onlyfile](../examples/Onlyfile).

## Contents

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
- [7. Select implementations with guards](#7-select-implementations-with-guards)
- [8. Choose a shell when needed](#8-choose-a-shell-when-needed)
  - [Run several lines in one shell](#run-several-lines-in-one-shell)
  - [Set a shell for the whole file](#set-a-shell-for-the-whole-file)
- [9. Organize larger projects with namespaces](#9-organize-larger-projects-with-namespaces)
- [10. Put the pieces together](#10-put-the-pieces-together)
- [Troubleshooting](#troubleshooting)
- [Diagnostics](#diagnostics)
- [Quick reference](#quick-reference)

---

## 1. Create your first Onlyfile

Create a file named `Onlyfile` in your project root:

```Onlyfile
!version 0.2

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
only -p
```

You can also select a file yourself:

```bash
only -f ./examples/Onlyfile --dry-run ci
```

### About `!version`

`!version` tells `only` the minimum language version your file needs.

```Onlyfile
!version 0.2
```

The format is always:

```text
MAJOR.MINOR
```

For example, `!version 0.2` requires language capability `0.2` or newer within the same major version.

The declaration is optional. Older Onlyfiles without `!version` remain valid.

If you use it, `!version` must be the first declaration in the file. Blank lines and `//` comments may appear before it.

---

## 2. Document your tasks

Use `//` for normal comments:

```Onlyfile
// This is only a source comment.
check():
    cargo check
```

Use `#` to document the next task:

```Onlyfile
# Check the project.
check():
    cargo check
```

Task documentation appears in `only` and `only --help`.

This lets the `Onlyfile` describe its own commands without separate metadata.

---

## 3. Add parameters

Add parameters inside the task signature:

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

`{{profile}}` inserts the bound parameter value into the command.

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

Use `..` to capture all remaining positional arguments:

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
only -s host=0.0.0.0 serve
```

Override more than one value:

```bash
only -s host=0.0.0.0 -s port=8080 serve
```

`-s` is a global option, so place it before the task path.

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

Each `&` creates another dependency stage.

Dependencies are tasks, not copied command text. If several tasks depend on the same task, `only` resolves that shared dependency once in the execution graph.

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
only --dry-run ci
```

To include rendered commands:

```bash
only --dry-run --full ci
```

For larger workflows, dry-run shows the stages after dependency and guard resolution.

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
- cannot be invoked directly.

This is useful for setup steps and internal workflow details.

---

## 7. Select implementations with guards

A task can have more than one implementation.

Use a guard to select the implementation that matches the current environment:

```Onlyfile
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test
```

If `cargo-nextest` exists on `PATH`, the first variant is used.

Otherwise, the unguarded task is the fallback.

You can also check the operating system:

```Onlyfile
package() ? @os("windows"):
    echo "Package for Windows"

package():
    echo "Package for Unix"
```

Available probes include:

| Probe | Matches when |
| --- | --- |
| `@os("macos")` | the current OS is macOS |
| `@os("linux")` | the current OS is Linux |
| `@os("windows")` | the current OS is Windows |
| `@arch("x86_64")` | the current architecture is x86-64 |
| `@arch("aarch64")` | the current architecture is AArch64 |
| `@env("CI")` | the environment variable exists |
| `@has("cargo")` | the command exists on `PATH` |

Variant selection follows three rules:

1. the first matching guarded variant wins;
2. if no guard matches, the unguarded variant is used;
3. if nothing matches and there is no fallback, the task is unavailable.

Guards let you choose an implementation without moving platform or tool detection into shell scripts.

---

## 8. Choose a shell when needed

By default, commands use the built-in cross-platform `deno` shell.

For common tasks, you normally do not need to choose a shell:

```Onlyfile
check():
    cargo check
```

Choose a host shell only when your command needs it.

Require Bash:

```Onlyfile
build() shell=bash:
    ./scripts/build.sh
```

Use `shell~=` when a compatible fallback is acceptable:

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

Normal command lines are separate execution units.

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
| `!version MAJOR.MINOR` | declare the minimum language capability |
| `!shell NAME` | set the default shell |

---

## 9. Organize larger projects with namespaces

Use namespaces when several parts of your project have similar tasks.

```Onlyfile
!version 0.3

[front] {
    check():
        pnpm lint

    test():
        pnpm test
}

[back] {
    check():
        cargo check

    test():
        cargo test
}
```

Run a namespaced task by placing the namespace before the task:

```bash
only front check
only front test
only back check
only back test
```

Run only the namespace name to see its tasks:

```bash
only front
```

You can also refer to namespaced tasks from dependencies:

```Onlyfile
ci() & (front.test, back.test):
    echo "CI complete"
```

This lets you keep short task names inside each project area without losing a clear global structure.

---

## 10. Put the pieces together

You can combine tasks, parameters, guards, namespaces, and parallel dependencies without turning the file into a large script.

Here is a small frontend + Rust example:

```Onlyfile
!version 0.3

build(profile="dev"):
    cargo build --profile {{profile}}

ci() & (back.test, front.test):
    echo "CI complete"

[back] {
    test() ? @has("cargo-nextest"):
        cargo nextest run

    test():
        cargo test
}

[front] {
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
only --dry-run --full ci
```

The important idea is simple:

> You describe the dependency relationships. `only` derives the execution stages.

---

# Troubleshooting

## `only` is using the wrong file

Check the discovered file:

```bash
only -p
```

Or select one explicitly:

```bash
only -f ./Onlyfile ci
```

---

## A task does not appear in `only`

Check whether:

- the task starts with `_`;
- the task belongs to a namespace;
- the namespace contains only hidden helper tasks;
- the `Onlyfile` contains an error.

For a namespaced task, use:

```bash
only <namespace> <task>
```

---

## `-s` / `--set` does not work

Make sure:

1. the parameter exists;
2. the task uses the expected parameter;
3. `-s` appears before the task path.

For example:

```bash
only -s port=8080 serve
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
only -s host=0.0.0.0 serve
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

If you need a host shell, you can:

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

# Diagnostics

`only` validates an `Onlyfile` before executing commands.

The LSP uses stable diagnostic codes so editor integrations do not depend on exact message text.

The CLI keeps errors shorter and does not normally show these internal codes.

| Code | Meaning | What to do |
| --- | --- | --- |
| `parse.unexpected-token` | unexpected syntax | check punctuation, indentation, and the task header |
| `parse.malformed-task-header` | malformed task declaration | use a valid form such as `name():` |
| `semantic.duplicate-task` | duplicate task definition | rename it or make the guarded variant distinct |
| `semantic.duplicate-parameter` | a parameter appears more than once | rename one parameter |
| `semantic.undefined-dependency` | a dependency does not exist | define it or fix the name |
| `semantic.undefined-variable` | interpolation uses an unknown parameter | declare it or fix the name |
| `semantic.ambiguous-guard` | variants use the same guard | remove one or change the guard |
| `semantic.slice-parameter-position` | a slice parameter is not last | move `name..` to the end |
| `semantic.slice-parameter-default` | a slice has a default value | remove the default |
| `version.invalid-format` | version is not `MAJOR.MINOR` | use two numeric components |
| `version.pre-0.1-unsupported` | the declaration uses `0.0` | omit it or use `0.1` or newer |
| `version.range-overflow` | a version value cannot be represented | use a smaller valid version |
| `version.duplicate` | the file has more than one `!version` | keep one declaration |
| `version.not-first-declaration` | `!version` appears too late | move it to the file header |

## Runtime errors

Common runtime errors include:

| Message | Meaning |
| --- | --- |
| `task '<name>' is not defined` | the task does not exist |
| `helper task '<name>' cannot be invoked directly` | helper tasks are dependency-only |
| `task '<name>' is not available for this environment` | no variant matched |
| `missing required parameter '{{name}}'` | a required value was not supplied |
| `unknown parameter '<name>' for task '<task>'` | `-s` refers to an unknown parameter |
| `cyclic dependency detected: ...` | the dependency graph contains a cycle |
| `unsupported shell '<name>'` | the shell name is not supported |
| `<shell> not found...` | the selected shell is not available on `PATH` |

---

# Quick reference

## CLI

| Command | Purpose |
| --- | --- |
| `only` | list available tasks |
| `only <task>` | run a root task |
| `only <namespace> <task>` | run a namespaced task |
| `only --help` | show help |
| `only <task> --help` | show task help and parameters |
| `only -p` / `only --path` | print the discovered `Onlyfile` path |
| `only -f <path>` / `only --file <path>` | use a specific file |
| `only -s name=value <task>` | override a parameter |
| `only --dry-run <task>` | show the execution plan |
| `only --dry-run --full <task>` | show the plan and rendered commands |
| `only -q <task>` / `only --quiet <task>` | hide progress lines but keep command output |

## Syntax

| Syntax | Meaning |
| --- | --- |
| `// text` | ordinary comment |
| `# text` | document the next task or namespace |
| `!version 0.2` | require language capability 0.2 or newer within 0.x |
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
| `[namespace] { ... }` | group tasks in a namespace |
