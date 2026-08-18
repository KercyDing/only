use only_syntax::{
    DirectiveKind, GuardKind, MetadataKind, ShellKind, ShellOperator, ShellSelection,
};

#[test]
fn builtins_round_trip() {
    for directive in DirectiveKind::SUPPORTED {
        assert_eq!(DirectiveKind::parse(directive.as_str()), *directive);
        assert!(directive.description().is_some());
    }
    for guard in GuardKind::SUPPORTED {
        assert_eq!(GuardKind::parse(guard.as_str()), *guard);
        assert!(guard.description().is_some());
    }
    for shell in ShellKind::SUPPORTED {
        assert_eq!(ShellKind::parse(shell.as_str()), *shell);
        assert!(shell.description().is_some());
    }
    for metadata in MetadataKind::SUPPORTED {
        assert_eq!(MetadataKind::parse(metadata.as_str()), *metadata);
        assert!(metadata.description().is_some());
    }
}

#[test]
fn unknown_builtins_keep_names() {
    assert_eq!(DirectiveKind::parse("custom").as_str(), "custom");
    assert_eq!(GuardKind::parse("custom").as_str(), "custom");
    assert_eq!(ShellKind::parse("custom").as_str(), "custom");
}

#[test]
fn shell_fallbacks_are_defined_once() {
    assert_eq!(ShellKind::Pwsh.fallback(), Some(ShellKind::Powershell));
    assert_eq!(ShellKind::Bash.fallback(), Some(ShellKind::Sh));
    assert_eq!(ShellKind::Sh.fallback(), None);

    let selection = ShellSelection::required(ShellKind::Deno);
    assert_eq!(selection.operator, ShellOperator::Required);
    assert_eq!(ShellOperator::Fallback.as_str(), "shell~=");
}
