# Changelog

## Unreleased

## 0.0.7 - 2026-07-13

- Added slice parameters with `name..` syntax to capture all remaining positional arguments.
- Added CLI support for forwarding unquoted trailing arguments, including values such as `--force`, into slice parameters.
- Added semantic diagnostics for slice parameters that are not final or that declare default values.
- Updated usage docs with slice parameter examples and diagnostics.

## 0.0.6 - 2026-07-11

- Removed the `!label`, `!echo`, and `!preview` file-level directives.
- Added `only --dry-run` to print a compact selected task plan without executing it.
- Added `only --dry-run --full` to expand rendered commands while keeping default dry-run output compact.
- Added `only -s` as the short form for `only --set`.
- Added `only -q` / `only --quiet` to hide only `only` progress lines while preserving command stdout and stderr.
- Changed Onlyfile comments to use `#` for task and namespace docs and standalone `//` lines for ordinary comments, making files easier to migrate from `just` without making line-end syntax ambiguous.
- Changed serial foreground tasks to inherit stdio so interactive and colored tools such as `cargo run` and `zig build run` behave like they do when run directly.
- Shortened task progress lines from `[task N/M] name` to `[N/M] name`.
- Rationale: output controls now live on the command line instead of in `Onlyfile`, because previewing and quieting are invocation-time choices. This keeps the task file focused on project semantics such as `!shell`, reduces configuration noise, and makes terminal-heavy commands feel less like dumped captured output.

## 0.0.5 - 2026-05-17

- Added `only --update` and `only --upgrade` for GitHub release based self updates.
- Expanded release artifacts to Linux, macOS, and Windows on x64 and ARM64.
- Fixed self-update replacement behavior and clarified Unix install errors.

## 0.0.4 - 2026-05-17

- Added the `!label` directive so command output labels can be disabled.
- Refreshed README, usage docs, and examples with the current directive and comment behavior.
- Tuned release profile settings to reduce binary size.
- Updated crate publish metadata and refreshed Cargo dependencies.

## 0.0.3 - 2026-04-23

- Rebuilt `only` as a multi-crate workspace with a clearer language pipeline.
- Added richer CLI behavior including dynamic help, namespace support, helper tasks, preview output, and parameter overrides.
- Added stronger execution features including guards, interpolation, echo control, and grouped parallel stages.
- Added an LSP server with diagnostics, hover, folding ranges, and document symbols.
- Expanded docs, examples, CI workflows, and regression coverage.
- Fixed helper-only namespace visibility, `!preview` hover docs, verbose CLI output, and `Onlyfile` string lexing.

## 0.0.2 - 2026-04-08

- First functional release of the `only` task runner.
- Added `Onlyfile` discovery, parsing, validation, execution planning, and runtime execution.
- Added dynamic CLI help, namespace support, task parameters, interpolation, and dependency resolution.
- Added cross-platform shell handling with Windows-specific command detection and task execution support.
- Added namespace-owned docs so `%` can describe `[namespace]` entries directly in task listings and help output.
- Added a project `Onlyfile` with `check`, `test`, `ci`, `install`, and `dev` / `rel` workflows, using `cargo-nextest` when available.
- Aligned README and usage docs with the currently implemented feature set.

## 0.0.1 - 2026-04-07

- Initial placeholder release with the project README and package metadata.
