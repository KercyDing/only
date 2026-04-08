# Changelog

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
