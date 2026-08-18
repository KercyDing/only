# Changelog

## 0.4.0 - 2026-08-18

- Added structured `[help]`, `[desc]`, `[pass]`, and `[fail]` metadata for tasks and groups.
- Added task success and failure messages, including file-level `!var` interpolation and guarded-variant inheritance.
- Replaced bracketed namespace groups with explicit `group name { ... }` declarations.
- Made `#` a normal source comment; it no longer supplies task help.
- Added metadata support across formatting, CLI help, diagnostics, completion, semantic tokens, and hover.

## 0.3.0 - 2026-08-17

- Added multiple guards, multiline task headers, file-level `!var` values, and braced namespace scopes.
- Replaced fallback shell selection with the unambiguous `shell~=` syntax.
- Added deterministic `only --fmt` and `only --fmt --check` formatting.
- Added LSP formatting and semantic tokens.

## 0.2.0 - 2026-08-17

- Added `|` command blocks that run consecutive marked lines in one shell process.
- Added block-aware dry-run output, runtime errors, folding, hover, and VS Code syntax support.

## 0.1.0 - 2026-08-17

- Added the `!version MAJOR.MINOR` bootstrap scanner and same-major language capability checks.
- Added dedicated format, placement, duplicate, runner, overflow, and incompatibility diagnostics across the CLI and LSP.
- Preserved unversioned Onlyfiles, added parse-failure guidance, and kept `only --path` independent from Onlyfile parsing.
- Simplified errors and stopped the CLI from repeating an error or showing later errors caused by it.
