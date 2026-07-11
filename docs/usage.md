# Usage

`only` is a cross-platform task runner driven by an `Onlyfile`. This guide starts small, then adds parameters, dependencies, guards, shells, namespaces, and troubleshooting.

## 1. Create your first Onlyfile

Create a file named `Onlyfile` in your project root:

```Onlyfile
# Minimal task.
hello():
    echo "hello from only"
```

Run it:

```bash
only hello
```

Run `only` with no task to list what the file exposes:

```bash
only
```

`only` discovers `Onlyfile` or `onlyfile` from the current directory upward. To see which file was found:

```bash
only -p
```

To use a specific file:

```bash
only -f ./examples/Onlyfile hello
```

## 2. Add task descriptions

Use standalone `//` lines for ordinary comments. They can appear between top-level items or inside a task body. Lines starting with `#` document the next task or namespace, and show up in `only` and `only --help` output.

```Onlyfile
// Ordinary comments are ignored.
# Format the codebase.
fmt():
    // Task-body comments are ignored too.
    cargo fmt --all

# Run tests.
test():
    cargo test
```

Now `only` gives a readable task list instead of just names.

## 3. Use parameters

Tasks use function-style signatures. A parameter without a default is required; a parameter with a default is optional.

```Onlyfile
# Serve the dev site.
serve(port="3000", host="127.0.0.1"):
    echo "Serving on {{host}}:{{port}}"
```

Run it with defaults:

```bash
only serve
# Serving on 127.0.0.1:3000
```

Pass positional arguments in declaration order:

```bash
only serve 8080
# Serving on 127.0.0.1:8080

only serve 8080 0.0.0.0
# Serving on 0.0.0.0:8080
```

Override by name with `-s` / `--set`:

```bash
only -s host=0.0.0.0 serve
# Serving on 0.0.0.0:3000

only -s host=0.0.0.0 serve 8080
# Serving on 0.0.0.0:8080
```

`-s` is a global option, so put it before the task path. It can be repeated:

```bash
only -s host=0.0.0.0 -s port=8080 serve
```

Parameter binding precedence is:

1. `-s NAME=VALUE` / `--set NAME=VALUE`
2. positional arguments
3. defaults from the task signature

### Required parameters

```Onlyfile
greet(name):
    echo "Hello, {{name}}"
```

```bash
only greet Ada
# Hello, Ada
```

Running `only greet` fails before any command runs because `name` has no default.

### Interpolation escapes

Use `\{{` and `\}}` when you need literal `{{` and `}}` in a command.

```Onlyfile
show(template="value"):
    echo "{{template}} and literal \{{template\}}"
```

```bash
only show
# value and literal {{template}}
```

## 4. Chain tasks with dependencies

Use `&` to run dependencies before the task body.

```Onlyfile
fmt():
    cargo fmt --all --check

check():
    cargo check

ci() & fmt & check:
    echo "CI complete"
```

`only ci` runs `fmt`, then `check`, then `ci`.

## 5. Run dependency groups in parallel

Use parentheses to run a stage in parallel.

```Onlyfile
build():
    cargo build --release

package():
    echo "package"

publish():
    echo "publish"

release() & build & (package, publish):
    echo "Release complete"
```

`build` runs first. After it succeeds, `package` and `publish` run together. The `release` body runs after both finish.

## 6. Hide helper tasks

A task whose name starts with `_` is a helper task. It can be used as a dependency, but it is hidden from normal listings and cannot be invoked directly.

```Onlyfile
_prepare():
    cargo fmt --all --check

ci() & _prepare:
    cargo test
```

Use this for internal steps that are useful inside workflows but noisy in `only` output.

## 7. Pick tasks with guards

A guard selects a task variant only when the current machine matches the probe.

```Onlyfile
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test
```

`only test` uses `cargo nextest run` when `cargo-nextest` exists on `PATH`; otherwise it falls back to `cargo test`.

Supported probes:

| Probe | Matches when |
| --- | --- |
| `@os("macos")` / `@os("linux")` / `@os("windows")` | `std::env::consts::OS` equals the argument |
| `@arch("x86_64")` / `@arch("aarch64")` | `std::env::consts::ARCH` equals the argument |
| `@env("CI")` | the environment variable is set |
| `@has("cargo")` | the command exists on `PATH` |

Selection rules:

- The first matching guarded variant wins.
- If no guarded variant matches, the unguarded variant is used as fallback.
- If there is no fallback, the task is unavailable on this machine.

## 8. Use a specific shell when needed

By default, commands run through the built-in cross-platform `deno` shell. That is what gives `only` consistent behavior across platforms.

Use a task-level shell only when a command really needs a host shell:

```Onlyfile
install() ? @os("windows") shell?=pwsh:
    Write-Output "Installing on Windows"

install():
    cargo install --path crates/cli --force
```

`shell=` requires that exact shell. `shell?=` prefers that shell, but allows a known compatible fallback:

- `shell?=pwsh` can fall back to `powershell`
- `shell?=bash` can fall back to `sh`

Accepted shell names:

| Shell | Behavior |
| --- | --- |
| `deno` | built-in cross-platform shell |
| `bash` | runs `bash -c` |
| `sh` | runs `sh -c` |
| `pwsh` | runs PowerShell 7+ |
| `powershell` | runs Windows PowerShell |

You can also set a file-level default shell:

```Onlyfile
!shell bash

hello():
    echo "hello from bash"
```

Resolution order is:

1. task-level `shell=` or `shell?=`
2. file-level `!shell`
3. built-in default `deno`

## 9. Configure file-level behavior with directives

Directives start with `!` and apply to the whole file.

```Onlyfile
!shell deno
```

| Directive | Values | Default | Effect |
| --- | --- | --- | --- |
| `!shell` | shell name | `deno` | Sets the default shell for tasks without an explicit shell. |

Use `only --dry-run <task>` when you want to see which task variant and dependency stages will run after parameters are bound. Add `--full` when you also need the rendered commands.

## 10. Organize tasks with namespaces

Namespaces group related tasks.

```Onlyfile
# Development workflow.
[dev]

# Build in development mode.
build():
    cargo build

# Run in development mode.
run():
    cargo run

# Release workflow.
[rel]

build():
    cargo build --release
```

Run namespaced tasks by putting the namespace before the task:

```bash
only dev build
only dev run
only rel build
```

Run `only dev` to see help for just that namespace.

## 11. A complete example

```Onlyfile
# Serve the dev site.
serve(port="3000", host="127.0.0.1"):
    echo "Serving on {{host}}:{{port}}"

# Format and check the workspace.
check():
    cargo fmt --all --check
    cargo check

# Prefer nextest when available.
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test

_package():
    echo "Packaging release"

publish():
    echo "Publishing release"

# Run the local CI workflow.
ci() & check & test:
    echo "CI complete"

# Build first, then package and publish together.
release() & check & (_package, publish):
    echo "Release complete"

# Development commands.
[dev]

build():
    cargo build

run():
    cargo run
```

Useful calls:

```bash
only
only serve
only -s host=0.0.0.0 serve 8080
only ci
only release
only dev build
```

## Troubleshooting by symptom

### `only` is using the wrong file

Run:

```bash
only -p
```

If the path is not what you expect, pass one explicitly:

```bash
only -f ./Onlyfile ci
```

### A task does not show up in `only`

Common causes:

- the task starts with `_`, so it is a helper
- the task is inside a namespace, so call it as `only <namespace> <task>`
- the namespace only contains helper tasks, so the namespace is hidden from the top-level list
- the file has parse or semantic errors, so the task was never registered

### `-s` / `--set` does not work

Check three things:

1. The task signature declares the parameter.
2. The command body uses `{{name}}`, not `$name`.
3. `-s` is written before the task path:

```bash
only -s port=8080 serve
```

### A positional argument sets the wrong value

Positionals bind in signature order.

```Onlyfile
serve(port="3000", host="127.0.0.1"):
    echo "{{host}}:{{port}}"
```

`only serve 8080` sets `port`, not `host`. Use `-s host=...` when you want to skip earlier parameters.

### A guarded task is unavailable

All guarded variants failed to match and there was no unguarded fallback. Add a fallback:

```Onlyfile
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test
```

### A shell is missing

Use the built-in `deno` shell when possible. If you need a host shell, install it or pick one that exists on `PATH`.
Use `shell?=` when the fallback is acceptable; use `shell=` when the exact shell is required.

```Onlyfile
build() shell?=bash:
    ./scripts/build.sh
```

Accepted names are `deno`, `bash`, `sh`, `pwsh`, and `powershell`.

### Interpolation fails

- `{{name}}` must match a declared parameter.
- Every unescaped `{{` needs a matching `}}`.
- Use `\{{` and `\}}` for literal braces.

## Diagnostics reference

`only` validates a file before running commands. Diagnostic codes are stable enough to search for.

### Parse diagnostics

| Code | Meaning | Fix |
| --- | --- | --- |
| `parse.unexpected-token` | grammar did not expect the current token | check punctuation, indentation, and header shape |
| `parse.malformed-directive` | broken `!` directive line | use `!shell <name>` |
| `parse.malformed-namespace-header` | broken `[namespace]` line | keep the namespace header on its own line |
| `parse.malformed-task-header` | broken task signature | use `name():` or `name(param="default"):` |

### Lower diagnostics

| Code | Meaning | Fix |
| --- | --- | --- |
| `lower.invalid-directive` | unknown directive or invalid directive value | use `!shell <name>` |
| `lower.invalid-task` | task header could not become a typed task | check parameters, guard, shell, and trailing colon |
| `lower.invalid-namespace` | namespace header could not become a typed namespace | use plain `[name]` |
| `lower.invalid-guard` | guard shape is invalid | use `@os(...)`, `@arch(...)`, `@env(...)`, or `@has(...)` |

### Semantic diagnostics

| Code | Meaning | Fix |
| --- | --- | --- |
| `semantic.duplicate-directive` | same directive appears twice | keep one copy |
| `semantic.duplicate-task` | duplicate task signature | rename it or make the variant distinct |
| `semantic.duplicate-parameter` | same parameter appears twice | rename one parameter |
| `semantic.ambiguous-guard` | two variants use the same guard | remove one or change the guard |
| `semantic.namespace-conflict` | namespace and global task share a name | rename one |
| `semantic.undefined-dependency` | dependency task is not defined | define it or fix the spelling |
| `semantic.undefined-variable` | `{{name}}` is not a parameter | declare the parameter or fix the interpolation |

### Runtime messages

| Message shape | Meaning |
| --- | --- |
| `task '<name>' is not defined` | no such task in the selected file |
| `helper task '<name>' cannot be invoked directly` | helpers are dependency-only |
| `task '<name>' is not available for this environment` | guards did not match and no fallback exists |
| `missing required parameter '{{name}}'` | required parameter was not supplied |
| `unknown parameter '<name>' for task '<task>'` | `-s` / `--set` targeted a non-existent parameter |
| `duplicate parameter override '<name>'` | same override name appeared twice |
| `cyclic dependency detected: ...` | dependencies form a cycle |
| `unterminated interpolation expression` | `{{` has no closing `}}` |
| `unsupported shell '<name>'` | shell name is not accepted |
| `<shell> not found...` | selected shell is not available on `PATH` |

## Quick reference

### CLI

| Command | Purpose |
| --- | --- |
| `only` | list available tasks |
| `only <task>` | run a global task |
| `only <namespace> <task>` | run a namespaced task |
| `only --help` | show help |
| `only <task> --help` | show task help, including parameters |
| `only -p` / `only --path` | print discovered `Onlyfile` path |
| `only -f <path>` / `only --file <path>` | use a specific file |
| `only -s name=value <task>` / `only --set name=value <task>` | override a task parameter |
| `only --dry-run <task>` | print the selected task plan without executing it; requires an explicit task |
| `only --dry-run --full <task>` | include rendered commands in dry-run output |
| `only -q <task>` / `only --quiet <task>` | hide progress lines while preserving command output |

### Syntax pieces

| Syntax | Meaning |
| --- | --- |
| `// text` | standalone ordinary comment |
| `# text` | document the next task or namespace |
| `!shell bash` | set file-level default shell |
| `task():` | define a task |
| `task(name="value"):` | define a parameter with default |
| `task(name):` | define a required parameter |
| `{{name}}` | interpolate a parameter |
| `_task():` | helper task |
| `task() & a & b:` | serial dependencies |
| `task() & a & (b, c):` | parallel dependency stage |
| `task() ? @has("cmd"):` | guarded task variant |
| `task() shell=bash:` | require an exact task shell |
| `task() shell?=bash:` | prefer a task shell with fallback |
| `[namespace]` | namespace following tasks |
