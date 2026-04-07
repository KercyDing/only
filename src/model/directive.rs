use super::SourceSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    Verbose { value: bool, span: SourceSpan },
    Shell { shell: ShellKind, span: SourceSpan },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Deno,
    Sh,
    Bash,
    PowerShell,
    Pwsh,
}
