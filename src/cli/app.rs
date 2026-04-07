use anstyle::{AnsiColor as TermAnsiColor, Style as TermStyle};
use clap::builder::StyledStr;
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Arg, ArgAction, Command};

use crate::model::{Namespace, Onlyfile, TaskDefinition};

pub fn build_global() -> Command {
    Command::new("only")
        .about("A minimalist, deterministic task runner")
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

pub fn build(onlyfile: &Onlyfile) -> Command {
    let mut cmd = build_global();

    for task in &onlyfile.global_tasks {
        cmd = cmd.subcommand(build_task_command(task));
    }

    for namespace in &onlyfile.namespaces {
        cmd = cmd.subcommand(build_namespace_command(namespace));
    }

    cmd
}

/// Renders dynamic help from a parsed `Onlyfile`.
///
/// Args:
/// onlyfile: Parsed task document.
///
/// Returns:
/// Help text including dynamically discovered tasks and namespaces.
pub fn render_help(onlyfile: &Onlyfile) -> StyledStr {
    let mut cmd = build(onlyfile);
    cmd.render_help()
}

/// Renders the compact task list shown by `only` with no task target.
///
/// Args:
/// onlyfile: Parsed task document.
///
/// Returns:
/// User-facing task list with global tasks and namespaces.
pub fn render_available_tasks(onlyfile: &Onlyfile) -> String {
    let entries = onlyfile
        .global_tasks
        .iter()
        .map(|task| {
            (
                task.signature.name.clone(),
                task.doc.clone().unwrap_or_default(),
                false,
            )
        })
        .chain(
            onlyfile
                .namespaces
                .iter()
                .map(|namespace| (namespace.name.clone(), namespace_summary(namespace), true)),
        )
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
/// namespace: Parsed namespace definition.
///
/// Returns:
/// Help text for the namespace subcommand.
pub fn render_namespace_help(namespace: &Namespace) -> StyledStr {
    let mut cmd = build_namespace_command(namespace);
    cmd.render_help()
}

/// Renders bootstrap help used before `Onlyfile` discovery succeeds.
///
/// Returns:
/// Global help text for the `only` entry point.
pub fn render_global_help() -> StyledStr {
    let mut cmd = build_global();
    cmd.render_help()
}

fn build_namespace_command(namespace: &Namespace) -> Command {
    let name: &'static str = Box::leak(namespace.name.clone().into_boxed_str());
    let mut cmd = Command::new(name)
        .bin_name(format!("only {}", namespace.name))
        .disable_help_subcommand(true)
        .styles(cli_styles())
        .about(namespace_summary(namespace));

    for task in &namespace.tasks {
        cmd = cmd.subcommand(build_task_command(task));
    }

    cmd
}

fn namespace_summary(namespace: &Namespace) -> String {
    namespace
        .tasks
        .iter()
        .find_map(|task| task.doc.clone())
        .unwrap_or_else(|| "Namespace".to_string())
}

fn build_task_command(task: &TaskDefinition) -> Command {
    let about = task.doc.clone().unwrap_or_default();
    let name: &'static str = Box::leak(task.signature.name.clone().into_boxed_str());
    let mut cmd = Command::new(name).styles(cli_styles()).about(about);

    for (index, param) in task.signature.parameters.iter().enumerate() {
        let pname: &'static str = Box::leak(param.name.clone().into_boxed_str());
        let arg = if let Some(default) = &param.default_value {
            let help = format!("Parameter (default: {default})");
            Arg::new(pname).index(index + 1).required(false).help(help)
        } else {
            Arg::new(pname)
                .index(index + 1)
                .required(true)
                .help("Required parameter")
        };
        cmd = cmd.arg(arg);
    }

    cmd
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
    use clap::error::ErrorKind;

    use crate::parse_onlyfile;

    use super::{
        build, render_available_tasks, render_global_help, render_help, render_namespace_help,
    };

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

        let mut cmd = build(&document);
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

        let matches = build(&document)
            .try_get_matches_from(["only", "dev", "--help"])
            .expect_err("help should short-circuit parsing");

        assert_eq!(matches.kind(), ErrorKind::DisplayHelp);
    }

    #[test]
    fn renders_bootstrap_help_without_tasks() {
        let help = render_global_help().to_string();

        assert!(help.contains("Usage: only [OPTIONS] [TASK] [ARGS]..."));
        assert!(help.contains("--file"));
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

        let help = render_namespace_help(&document.namespaces[0]).to_string();
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
        assert!(listing.contains("Default developer workflow."));
    }
}
