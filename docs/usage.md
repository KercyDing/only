# Usage

`only` is a cross-platform task runner driven by an `Onlyfile`.

This guide starts with a minimal task, then introduces parameters, dependencies, parallel execution, helpers, guards, shells, namespaces, diagnostics, and CLI reference.

## 1. Create your first Onlyfile

Create an `Onlyfile` in your project root:

```Onlyfile
# Minimal task.
hello():
    echo "hello from only"
```

Run it:

```bash
only hello
```

Run `only` without a task to list available commands:

```bash
only
```

`only` searches for `Onlyfile` or `onlyfile` from the current directory upward.

Print the discovered path:

```bash
only -p
```

Or use a specific file:

```bash
only -f ./examples/Onlyfile hello
```

### Declare the language version

An Onlyfile can declare the minimum language capability it requires:

```Onlyfile
!version 0.1

# Minimal task.
hello():
    echo "hello from only"
```

The declaration is optional. Existing files without it remain valid and do not produce a warning.

`!version` accepts exactly `MAJOR.MINOR`, must be the first declaration, and may only be preceded by a UTF-8 BOM, blank lines, or `//` comments. For example, `!version 0.1` accepts stable runners from `0.1.0` through later `0.x` releases and rejects `1.x`. `!version 0.0` is invalid because the compatibility protocol begins at `0.1`.

## 2. Add task descriptions

Use `//` for ordinary comments.

Use `#` to document the next task or namespace. These descriptions appear in `only` and `only --help`.

```Onlyfile
// Ordinary comments are ignored.

# Format the codebase.
fmt():
    cargo fmt --all

# Run tests.
test():
    cargo test
```

This keeps the task list readable without adding separate metadata.

## 3. Use parameters

Tasks use function-style signatures.

A parameter without a default is required. A parameter with a default is optional.

```Onlyfile
# Serve the dev site.
serve(port="3000", host="127.0.0.1"):
    echo "Serving on {{host}}:{{port}}"
```

Use the defaults:

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

Override parameters by name with `-s` / `--set`:

```bash
only -s host=0.0.0.0 serve
# Serving on 0.0.0.0:3000

only -s host=0.0.0.0 serve 8080
# Serving on 0.0.0.0:8080
```

`-s` is a global option, so place it before the task path. It can be repeated:

```bash
only -s host=0.0.0.0 -s port=8080 serve
```

Parameter precedence is:

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

Running `only greet` fails before any command executes because `name` has no default.

### Slice parameters

Use a slice parameter to capture all remaining positional arguments:

```Onlyfile
run(args..):
    cargo run {{args}}
```

```bash
only run --release --bin demo
```

`args..` binds to `--release --bin demo`.

A slice may follow fixed parameters:

```Onlyfile
tool(name, args..):
    {{name}} {{args}}
```

Slice parameters:

- accept zero or more arguments;
- must be the final parameter;
- cannot have a default value.

### Interpolation escapes

Use `\{{` and `\}}` when literal braces are needed:

```Onlyfile
show(template="value"):
    echo "{{template}} and literal \{{template\}}"
```

```bash
only show
# value and literal {{template}}
```

## 4. Declare dependencies

Use `&` to declare ordered dependency stages before the task body.

```Onlyfile
fmt():
    cargo fmt --all --check

check():
    cargo check

ci() & fmt & check:
    echo "CI complete"
```

Running:

```bash
only ci
```

executes:

```text
fmt
 ↓
check
 ↓
ci
```

Dependencies are tasks, not textually expanded command blocks. Shared dependencies are resolved once within the execution graph.

## 5. Run dependencies in parallel

Use parentheses to declare dependencies that may run in parallel.

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

The dependency relationship is:

```text
build
  │
  ├─ package ─┐
  └─ publish ─┴─ release
```

`build` runs first. After it succeeds, `package` and `publish` may run together. The `release` body runs after both finish.

Compare:

```Onlyfile
ci() & check & test:
```

```text
check → test → ci
```

with:

```Onlyfile
ci() & (check, test):
```

```text
check ─┐
       ├→ ci
test ──┘
```

### Inspect the execution plan

Use dry-run to see the resolved task stages without executing them:

```bash
only --dry-run release
```

Add `--full` to include rendered commands:

```bash
only --dry-run --full release
```

For larger workflows, this shows the actual schedule after dependency and guard resolution.

## 6. Hide helper tasks

A task whose name starts with `_` is a helper task.

Helpers may be used as dependencies, but they are hidden from normal listings and cannot be invoked directly.

```Onlyfile
_prepare():
    cargo fmt --all --check

ci() & _prepare:
    cargo test
```

Helpers are useful for setup and internal workflow steps that should not become public commands.

## 7. Select implementations with guards

A task may have multiple variants.

Guards select the first variant whose probe matches the current environment.

```Onlyfile
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test
```

When `cargo-nextest` exists on `PATH`:

```bash
only test
# cargo nextest run
```

Otherwise the unguarded variant is used:

```bash
only test
# cargo test
```

Supported probes:

| Probe | Matches when |
| --- | --- |
| `@os("macos")` / `@os("linux")` / `@os("windows")` | current operating system matches |
| `@arch("x86_64")` / `@arch("aarch64")` | current architecture matches |
| `@env("CI")` | environment variable exists |
| `@has("cargo")` | command exists on `PATH` |

Selection rules:

1. the first matching guarded variant wins;
2. if none match, the unguarded variant is used as fallback;
3. without a matching guard or fallback, the task is unavailable.

For example:

```Onlyfile
package() ? @os("windows"):
    echo "Package Windows build"

package() ? @os("macos"):
    echo "Package macOS build"

package():
    echo "Package Unix build"
```

## 8. Choose a shell when needed

Commands use the built-in cross-platform `deno` shell by default.

This lets common commands behave consistently across macOS, Linux, and Windows without duplicating task definitions.

Use a task-level shell only when a command specifically requires one:

```Onlyfile
install() ? @os("windows") shell?=pwsh:
    Write-Output "Installing on Windows"

install():
    cargo install --path crates/cli --force
```

`shell=` requires the exact shell:

```Onlyfile
build() shell=bash:
    ./scripts/build.sh
```

`shell?=` prefers the requested shell but allows a compatible fallback:

```Onlyfile
build() shell?=bash:
    ./scripts/build.sh
```

Known fallbacks:

- `pwsh` → `powershell`
- `bash` → `sh`

### Run several lines in one shell

Prefix consecutive lines with `|` to run them as one command block:

```Onlyfile
!version 0.2

check() shell=bash:
    | if test -f Cargo.lock; then
    |     cargo check --locked
    | fi
    echo "done"
```

The block shares shell state. The following ordinary command starts a new shell. Use a bare `|` for a blank line inside a block.

Accepted shells:

| Shell | Behavior |
| --- | --- |
| `deno` | built-in cross-platform shell |
| `bash` | runs `bash -c` |
| `sh` | runs `sh -c` |
| `pwsh` | runs PowerShell 7+ |
| `powershell` | runs Windows PowerShell |

### Set a file-level shell

Use the `!shell` directive to change the default for the whole file:

```Onlyfile
!shell bash

hello():
    echo "hello from bash"
```

Shell resolution order:

1. task-level `shell=` or `shell?=`
2. file-level `!shell`
3. built-in `deno`

Supported file-level directives:

| Directive | Values | Default | Effect |
| --- | --- | --- | --- |
| `!version` | `MAJOR.MINOR` | none | require a compatible Onlyfile language version |
| `!shell` | shell name | `deno` | default shell for tasks without an explicit shell |

## 9. Organize tasks with namespaces

Namespaces group related tasks without encoding the group into every task name.

```Onlyfile
# Frontend workflow.
[front]

# Check the frontend.
check():
    pnpm lint

# Test the frontend.
test():
    pnpm test

# Backend workflow.
[back]

# Check the backend.
check():
    cargo clippy

# Test the backend.
test():
    cargo test
```

Run namespaced tasks by placing the namespace before the task:

```bash
only front check
only front test
only back check
only back test
```

Run:

```bash
only front
```

to show help for that namespace.

Namespaces may also be used in dependencies:

```Onlyfile
ci() & (front.ci, back.ci):
    echo "CI complete"
```

## 10. Put it together

The following is a simplified workflow from a Tauri-style frontend + Rust project:

```Onlyfile
# Run local CI.
ci() & (front.ci, back.ci):
    echo "CI complete"

// =============== Frontend ===============

[front]

_prepare() ? @has("pnpm"):
    pnpm install

_prepare():
    echo "pnpm is required"
    exit 1

check() & _prepare:
    pnpm fmt:check
    pnpm lint
    pnpm build:check

test() & _prepare:
    pnpm test

# Check and test in parallel.
ci() & (check, test):
    echo "Frontend CI complete"

// =============== Backend ===============

[back]

_prepare() ? @has("cargo"):
    echo "Rust toolchain ready"

_prepare():
    echo "cargo is required"
    exit 1

check() & _prepare:
    cargo fmt --all -- --check
    cargo check --workspace --all-targets
    cargo clippy --workspace --all-targets -- -D warnings

test() ? @has("cargo-nextest") & _prepare:
    cargo nextest run --no-fail-fast

test() & _prepare:
    cargo test --workspace --all-targets

# Check, then test.
ci() & check & test:
    echo "Backend CI complete"
```

Inspect it:

```bash
only --dry-run --full ci
```

A matching environment may resolve to:

```text
Dry run: ci()
├─ stage 1 (parallel)
│  ├─ front._prepare
│  │  └─ pnpm install
│  └─ back._prepare
│     └─ echo "Rust toolchain ready"
├─ stage 2 (parallel)
│  ├─ front.check
│  ├─ front.test
│  └─ back.check
├─ stage 3 (parallel)
│  ├─ front.ci
│  └─ back.test
├─ stage 4
│  └─ back.ci
└─ stage 5
   └─ ci
```

`only` resolves guards, deduplicates shared dependencies, builds the task graph, and schedules independent tasks together.

You describe the dependency relationships; `only` derives the execution stages.

## Troubleshooting

### `only` is using the wrong file

Print the discovered path:

```bash
only -p
```

Or pass the file explicitly:

```bash
only -f ./Onlyfile ci
```

### A task does not appear in `only`

Common causes:

- the task starts with `_`, so it is a helper;
- the task belongs to a namespace;
- the namespace contains only helper tasks and is hidden from the top-level list;
- the file contains parse or semantic errors.

For a namespaced task:

```bash
only <namespace> <task>
```

### `-s` / `--set` does not work

Check that:

1. the task declares the parameter;
2. the body uses `{{name}}`;
3. `-s` appears before the task path.

```bash
only -s port=8080 serve
```

### A positional argument sets the wrong parameter

Positionals bind in declaration order.

```Onlyfile
serve(port="3000", host="127.0.0.1"):
    echo "{{host}}:{{port}}"
```

Therefore:

```bash
only serve 8080
```

sets `port`.

To skip an earlier parameter, use a named override:

```bash
only -s host=0.0.0.0 serve
```

### A guarded task is unavailable

No guarded variant matched and no unguarded fallback exists.

Add a fallback when appropriate:

```Onlyfile
test() ? @has("cargo-nextest"):
    cargo nextest run

test():
    cargo test
```

### A shell is missing

Prefer the built-in `deno` shell when possible.

If a host shell is required:

- install it;
- choose another supported shell;
- use `shell?=` when a compatible fallback is acceptable.

```Onlyfile
build() shell?=bash:
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

### Interpolation fails

Check that:

- `{{name}}` matches a declared parameter;
- every unescaped `{{` has a closing `}}`;
- literal braces use `\{{` and `\}}`.

## Diagnostics

`only` validates an `Onlyfile` before executing commands.

LSP diagnostics include stable codes so editor integrations do not depend on exact wording. The CLI keeps errors concise and does not print these internal codes.

| Code | Meaning | Fix |
| --- | --- | --- |
| `parse.unexpected-token` | unexpected syntax | check punctuation, indentation, and task header shape |
| `parse.malformed-task-header` | malformed task declaration | use `name():` or `name(param="value"):` |
| `semantic.duplicate-task` | duplicate task definition | rename it or make the guarded variant distinct |
| `semantic.duplicate-parameter` | parameter appears more than once | rename one parameter |
| `semantic.undefined-dependency` | referenced dependency does not exist | define it or fix the spelling |
| `semantic.undefined-variable` | interpolation references an undeclared parameter | declare it or fix the name |
| `semantic.ambiguous-guard` | multiple variants use the same guard | remove one or change the guard |
| `semantic.slice-parameter-position` | slice parameter is not last | move `name..` to the end |
| `semantic.slice-parameter-default` | slice parameter has a default | remove the default |
| `version.invalid-format` | version is not exactly `MAJOR.MINOR` | use two numeric components without leading zeros |
| `version.pre-0.1-unsupported` | declaration uses `0.0` | omit the declaration or require `0.1` |
| `version.range-overflow` | a version component or upper bound overflows | use a representable version |
| `version.duplicate` | multiple version declarations exist | keep only the first declaration |
| `version.not-first-declaration` | version follows another declaration | move it to the file header |
| `version.incompatible` | runner is outside the required range | install a compatible runner |
| `version.invalid-runner-version` | runner version is not valid SemVer | reinstall a stable runner build |

### Runtime errors

Common runtime messages include:

| Message shape | Meaning |
| --- | --- |
| `task '<name>' is not defined` | no such task exists |
| `helper task '<name>' cannot be invoked directly` | helpers are dependency-only |
| `task '<name>' is not available for this environment` | no guarded variant matched and no fallback exists |
| `missing required parameter '{{name}}'` | required parameter was not supplied |
| `unknown parameter '<name>' for task '<task>'` | `-s` targeted an undeclared parameter |
| `cyclic dependency detected: ...` | dependency graph contains a cycle |
| `unsupported shell '<name>'` | shell name is not supported |
| `<shell> not found...` | selected shell is unavailable on `PATH` |

## Quick reference

### CLI

| Command | Purpose |
| --- | --- |
| `only` | list available tasks |
| `only <task>` | run a global task |
| `only <namespace> <task>` | run a namespaced task |
| `only --help` | show help |
| `only <task> --help` | show task help and parameters |
| `only -p` / `only --path` | print the discovered `Onlyfile` path |
| `only -f <path>` / `only --file <path>` | use a specific file |
| `only -s name=value <task>` / `only --set name=value <task>` | override a parameter |
| `only --dry-run <task>` | show the resolved execution plan |
| `only --dry-run --full <task>` | include rendered commands |
| `only -q <task>` / `only --quiet <task>` | hide progress lines while preserving command output |

### Syntax

| Syntax | Meaning |
| --- | --- |
| `// text` | ordinary comment |
| `# text` | document the next task or namespace |
| `!version 0.2` | require language capability 0.2 or newer within 0.x |
| `!shell bash` | set the file-level default shell |
| `task():` | define a task |
| `task(name):` | required parameter |
| `task(name="value"):` | parameter with default |
| `task(args..):` | slice parameter |
| `{{name}}` | interpolate a parameter |
| `_task():` | helper task |
| `task() & a & b:` | ordered dependency stages |
| `task() & a & (b, c):` | parallel dependency stage |
| `task() ? @has("cmd"):` | guarded task variant |
| `task() shell=bash:` | require an exact shell |
| `task() shell?=bash:` | prefer a shell with fallback |
| `| command` | continue a command block in one shell process |
| `[namespace]` | namespace following tasks |
