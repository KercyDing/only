use anstyle::{AnsiColor as TermAnsiColor, Style as TermStyle};
use clap::builder::StyledStr;
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Arg, ArgAction, Command};
use only_semantic::{DocumentAst, NamespaceAst, TaskAst};
use std::collections::HashSet;

/// Builds the global CLI skeleton shared by bootstrap and dynamic help.
///
/// Args:
/// None.
///
/// Returns:
/// Base clap command with host-level options.
pub fn build_global_cli() -> Command {
    Command::new("only")
        .about("A minimalist, deterministic task runner")
        .version(env!("CARGO_PKG_VERSION"))
        .styles(cli_styles())
        .disable_help_subcommand(true)
        .override_usage("only [OPTIONS] [TASK] [ARGS]...")
        .arg(
            Arg::new("onlyfile")
                .short('f')
                .long("file")
                .value_name("PATH")
                .global(true)
                .help("Use a specific Onlyfile path"),
        )
        .arg(
            Arg::new("print-path")
                .short('p')
                .long("path")
                .action(ArgAction::SetTrue)
                .global(true)
                .help("Print the resolved Onlyfile path and exit successfully"),
        )
        .arg(
            Arg::new("set")
                .long("set")
                .value_name("NAME=VALUE")
                .action(ArgAction::Append)
                .global(true)
                .help("Override a target task parameter"),
        )
}

/// Builds the full dynamic CLI from a parsed semantic document.
///
/// Args:
/// document: Parsed task document.
///
/// Returns:
/// Clap command with global tasks and namespaces wired as subcommands.
pub fn build_cli(document: &DocumentAst) -> Command {
    let mut cmd = build_global_cli();

    for task in unique_tasks(global_tasks(document)) {
        cmd = cmd.subcommand(build_task_command(task));
    }

    for namespace in &document.namespaces {
        cmd = cmd.subcommand(build_namespace_command(document, namespace));
    }

    cmd
}

/// Renders dynamic help from a parsed semantic document.
///
/// Args:
/// document: Parsed task document.
///
/// Returns:
/// Help text including dynamically discovered tasks and namespaces.
pub fn render_help(document: &DocumentAst) -> StyledStr {
    let mut cmd = build_cli(document);
    cmd.render_help()
}

/// Renders the compact task list shown by `only` with no task target.
///
/// Args:
/// document: Parsed task document.
///
/// Returns:
/// User-facing task list with global tasks and namespaces.
pub fn render_available_tasks(document: &DocumentAst) -> String {
    let entries = unique_tasks(global_tasks(document).filter(|task| !task.is_helper()))
        .into_iter()
        .map(|task| {
            (
                task.name.to_string(),
                task.doc
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                false,
            )
        })
        .chain(document.namespaces.iter().map(|namespace| {
            (
                namespace.name.to_string(),
                namespace_summary(namespace),
                true,
            )
        }))
        .collect::<Vec<_>>();

    if entries.is_empty() {
        return "Available tasks:\n".to_string();
    }

    let width = entries
        .iter()
        .map(|(name, _, is_group)| name.len() + if *is_group { 8 } else { 0 })
        .max()
        .unwrap_or_default();

    let header_style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightGreen.into()))
        .bold();
    let task_style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightCyan.into()))
        .bold();
    let group_style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightYellow.into()))
        .bold();

    let mut output = format!(
        "{}Available tasks:{}\n",
        header_style.render(),
        header_style.render_reset()
    );
    for (name, doc, is_group) in entries {
        let suffix = if is_group { " [group]" } else { "" };
        let padding = " ".repeat(width.saturating_sub(name.len() + suffix.len()));
        let group_marker = if is_group {
            format!(
                " {}[group]{}",
                group_style.render(),
                group_style.render_reset()
            )
        } else {
            String::new()
        };
        output.push_str(&format!(
            "  {}{}{}{}{} {doc}\n",
            task_style.render(),
            name,
            task_style.render_reset(),
            group_marker,
            padding
        ));
    }

    output
}

/// Renders help for a namespace and all of its child tasks.
///
/// Args:
/// document: Parsed task document.
/// namespace: Parsed namespace definition.
///
/// Returns:
/// Help text for the namespace subcommand.
pub fn render_namespace_help(document: &DocumentAst, namespace: &NamespaceAst) -> StyledStr {
    let mut cmd = build_namespace_command(document, namespace);
    cmd.render_help()
}

/// Renders bootstrap help used before `Onlyfile` discovery succeeds.
///
/// Args:
/// None.
///
/// Returns:
/// Global help text for the `only` entry point.
pub fn render_global_help() -> StyledStr {
    let mut cmd = build_global_cli();
    cmd.render_help()
}

/// Renders the top-level host error message.
///
/// Args:
/// message: Human-readable error body.
///
/// Returns:
/// Styled terminal error text.
pub fn render_error_message(message: &str) -> String {
    let label_style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightRed.into()))
        .bold();
    let body_style = TermStyle::new().fg_color(Some(TermAnsiColor::BrightRed.into()));

    format!(
        "{}Error:{} {}{}{}",
        label_style.render(),
        label_style.render_reset(),
        body_style.render(),
        message,
        body_style.render_reset()
    )
}

/// Renders the generic help hint shown after discovery failures.
///
/// Args:
/// None.
///
/// Returns:
/// Styled terminal hint text.
pub fn render_help_hint() -> String {
    let style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightCyan.into()))
        .bold();

    format!(
        "Run '{}only --help{}' to view usage.",
        style.render(),
        style.render_reset()
    )
}

fn build_namespace_command(document: &DocumentAst, namespace: &NamespaceAst) -> Command {
    let mut cmd = Command::new(namespace.name.to_string())
        .bin_name(format!("only {}", namespace.name))
        .disable_help_subcommand(true)
        .styles(cli_styles());

    if let Some(doc) = &namespace.doc {
        cmd = cmd.about(doc.to_string());
    }

    for task in unique_tasks(namespace_tasks(document, namespace.name.as_str())) {
        cmd = cmd.subcommand(build_task_command(task));
    }

    cmd
}

fn namespace_summary(namespace: &NamespaceAst) -> String {
    namespace
        .doc
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default()
}

fn build_task_command(task: &TaskAst) -> Command {
    let about = task
        .doc
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut cmd = Command::new(task.name.to_string())
        .styles(cli_styles())
        .about(about)
        .hide(task.is_helper());

    for (index, param) in task.params.iter().enumerate() {
        let arg = if let Some(default) = &param.default_value {
            let help = format!("Parameter (default: {default})");
            Arg::new(param.name.to_string())
                .index(index + 1)
                .required(false)
                .help(help)
        } else {
            Arg::new(param.name.to_string())
                .index(index + 1)
                .required(false)
                .help("Required parameter")
        };
        cmd = cmd.arg(arg);
    }

    cmd
}

fn unique_tasks<'a>(tasks: impl IntoIterator<Item = &'a TaskAst>) -> Vec<&'a TaskAst> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();

    for task in tasks {
        if seen.insert(task.name.as_str()) {
            unique.push(task);
        }
    }

    unique
}

fn global_tasks(document: &DocumentAst) -> impl Iterator<Item = &TaskAst> {
    document
        .tasks
        .iter()
        .filter(|task| task.namespace.is_none())
}

fn namespace_tasks<'a>(
    document: &'a DocumentAst,
    namespace: &'a str,
) -> impl Iterator<Item = &'a TaskAst> {
    document
        .tasks
        .iter()
        .filter(move |task| task.namespace.as_deref() == Some(namespace))
}

fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::BrightGreen.on_default() | Effects::BOLD)
        .usage(AnsiColor::BrightGreen.on_default() | Effects::BOLD)
        .literal(AnsiColor::BrightCyan.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::BrightYellow.on_default())
        .valid(AnsiColor::BrightCyan.on_default())
        .invalid(AnsiColor::BrightRed.on_default() | Effects::BOLD)
        .error(AnsiColor::BrightRed.on_default() | Effects::BOLD)
}

#[cfg(test)]
mod tests {
    use super::{
        build_cli, build_global_cli, render_available_tasks, render_error_message,
        render_global_help, render_help, render_help_hint, render_namespace_help,
    };
    use crate::parse_onlyfile;
    use clap::error::ErrorKind;
    use std::panic;

    #[test]
    fn renders_colored_error_message() {
        let rendered = render_error_message("task failed");
        assert!(rendered.contains("Error:"));
        assert!(rendered.contains("task failed"));
    }

    #[test]
    fn renders_help_hint() {
        let rendered = render_help_hint();
        assert!(rendered.contains("only --help"));
    }

    #[test]
    fn renders_namespace_entries_with_trailing_slash() {
        let document = parse_onlyfile(
            "[dev]
% Default developer workflow.
workflow():
    echo ok
",
        )
        .expect("document should parse");

        let mut cmd = build_cli(&document);
        let help = cmd.render_help().to_string();

        assert!(help.contains("dev"));
        assert!(!help.contains("dev/"));
    }

    #[test]
    fn accepts_namespace_help_via_alias_without_trailing_slash() {
        let document = parse_onlyfile(
            "[dev]
% Default developer workflow.
workflow():
    echo ok
",
        )
        .expect("document should parse");

        let matches = build_cli(&document)
            .try_get_matches_from(["only", "dev", "--help"])
            .expect_err("help should short-circuit parsing");

        assert_eq!(matches.kind(), ErrorKind::DisplayHelp);
    }

    #[test]
    fn renders_bootstrap_help_without_tasks() {
        let help = render_global_help().to_string();

        assert!(help.contains("Usage: only [OPTIONS] [TASK] [ARGS]..."));
        assert!(help.contains("--file"));
        assert!(help.contains("--version"));
    }

    #[test]
    fn supports_global_version_flag() {
        let error = build_global_cli()
            .try_get_matches_from(["only", "--version"])
            .expect_err("version should short-circuit parsing");

        assert_eq!(error.kind(), ErrorKind::DisplayVersion);
    }

    #[test]
    fn renders_dynamic_root_help_with_tasks_and_namespaces() {
        let document = parse_onlyfile(
            "% Run tests.
test():
    cargo test

[dev]
% Default developer workflow.
workflow():
    echo ok
",
        )
        .expect("document should parse");

        let help = render_help(&document).to_string();
        assert!(help.contains("test"));
        assert!(help.contains("Run tests."));
        assert!(help.contains("dev"));
        assert!(!help.contains("dev/"));
    }

    #[test]
    fn renders_namespace_help_without_default_task() {
        let document = parse_onlyfile(
            "[dev]
% Default developer workflow.
workflow():
    echo ok

% Run a namespaced smoke command.
smoke():
    echo smoke
",
        )
        .expect("document should parse");

        let help = render_namespace_help(&document, &document.namespaces[0]).to_string();
        assert!(help.contains("Usage: only dev [COMMAND]"));
        assert!(help.contains("workflow"));
        assert!(help.contains("smoke"));
        assert!(!help.contains("\n  help"));
    }

    #[test]
    fn hides_help_subcommand_from_dynamic_help() {
        let document = parse_onlyfile(
            "% Run tests.
test():
    cargo test
",
        )
        .expect("document should parse");

        let help = render_help(&document).to_string();
        assert!(!help.contains("\n  help"));
    }

    #[test]
    fn renders_available_tasks_listing() {
        let document = parse_onlyfile(
            "% Run tests.
test():
    cargo test

[dev]
% Default developer workflow.
workflow():
    echo ok
",
        )
        .expect("document should parse");

        let listing = render_available_tasks(&document);
        assert!(listing.contains("Available tasks:"));
        assert!(listing.contains("test"));
        assert!(listing.contains("Run tests."));
        assert!(listing.contains("dev"));
        assert!(!listing.contains("Default developer workflow."));
    }

    #[test]
    fn renders_namespace_summary_from_namespace_doc() {
        let document = parse_onlyfile(
            "% Developer workflow.\n[dev]\n% Run smoke.\nsmoke():\n    echo smoke\n",
        )
        .expect("document should parse");

        let listing = render_available_tasks(&document);
        assert!(listing.contains("Developer workflow."));
        assert!(!listing.contains("Run smoke."));
    }

    #[test]
    fn omits_namespace_fallback_summary_when_doc_is_missing() {
        let document = parse_onlyfile(
            "[dev]
% Run smoke.
smoke():
    echo smoke
",
        )
        .expect("document should parse");

        let help = render_namespace_help(&document, &document.namespaces[0]).to_string();
        assert!(help.starts_with("Usage: only dev [COMMAND]"));
    }

    #[test]
    fn renders_namespace_help_about_from_namespace_doc() {
        let document = parse_onlyfile(
            "% Developer workflow.
[dev]
% Run smoke.
smoke():
    echo smoke
",
        )
        .expect("document should parse");

        let help = render_namespace_help(&document, &document.namespaces[0]).to_string();
        assert!(help.starts_with("Developer workflow."));
    }

    #[test]
    fn hides_helper_tasks_from_rendered_outputs() {
        let document = parse_onlyfile(
            "% Run tests.\n_test_helper():\n    cargo test\ntest():\n    cargo test\n\n[dev]\n_workflow():\n    echo hidden\nworkflow():\n    echo ok\n",
        )
        .expect("document should parse");

        let listing = render_available_tasks(&document);
        assert!(listing.contains("test"));
        assert!(!listing.contains("_test_helper"));
        assert!(!listing.contains("_workflow"));

        let root_help = render_help(&document).to_string();
        assert!(root_help.contains("test"));
        assert!(!root_help.contains("_test_helper"));

        let namespace_help = render_namespace_help(&document, &document.namespaces[0]).to_string();
        assert!(namespace_help.contains("workflow"));
        assert!(!namespace_help.contains("_workflow"));
    }

    #[test]
    fn allows_guarded_task_variants_without_duplicate_subcommand_panic() {
        let document = parse_onlyfile(
            r#"probe() ? @env("PATH"):
    true

probe():
    false
"#,
        )
        .expect("document should parse");

        let result = panic::catch_unwind(|| build_cli(&document));

        assert!(result.is_ok(), "building CLI should not panic");
    }
}
