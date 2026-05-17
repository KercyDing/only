# Changelog

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
