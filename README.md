# Only

A minimalist, deterministic task runner.

> Project is currently under active development.

## Notes

- `only` with no arguments lists available tasks when an `Onlyfile` is found.
- `only <namespace>` prints help for that namespace and its child tasks.
- `{{name}}` interpolation is plain text substitution. It does not perform shell escaping, so task authors must quote or escape parameters when needed.
