use anstyle::{AnsiColor as TermAnsiColor, Style as TermStyle};
use clap::builder::StyledStr;
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Arg, ArgAction, Command};
use only_engine::{PlanParam, render_command, select_root_task_variant};
use only_semantic::{DocumentAst, NamespaceAst, TaskAst};
use std::collections::HashSet;

const NAMESPACE_HELP_TEMPLATE: &str = "\
{about-with-newline}
{usage-heading} {usage}{after-help}

Run `only -h` to see available options.";

const TASK_HELP_TEMPLATE: &str = "\
{about-with-newline}
{usage-heading} {usage}

Run `only -h` to see available options.";

const TASK_HELP_WITH_ARGS_TEMPLATE: &str = "\
{about-with-newline}
{usage-heading} {usage}

{all-args}

Run `only -h` to see available options.";

/// Builds the global CLI skeleton shared by bootstrap and dynamic help.
///
/// Args:
/// None.
///
/// Returns:
/// Base clap command with host-level options.
pub fn build_global_cli() -> Command {
    let command = Command::new("only")
        .bin_name("only")
        .about(
            "An explicit, cross-platform workflow runner.\nRepo: https://github.com/KercyDing/only",
        )
        .version(env!("CARGO_PKG_VERSION"))
        .styles(cli_styles())
        .disable_help_subcommand(true)
        .override_usage("only [TASK] [ARGS]... [OPTIONS]");

    with_global_options(command, false)
        .arg(
            Arg::new("upgrade")
                .long("upgrade")
                .action(ArgAction::SetTrue)
                .help("Upgrade Only"),
        )
        .arg(
            Arg::new("update")
                .long("update")
                .action(ArgAction::SetTrue)
                .help("Same as --upgrade"),
        )
}

fn with_global_options(mut command: Command, hidden: bool) -> Command {
    let options = [
        Arg::new("onlyfile")
            .short('p')
            .long("path")
            .value_name("PATH")
            .global(true)
            .hide(hidden)
            .help("Use a specific Onlyfile"),
        Arg::new("print-path")
            .long("where")
            .action(ArgAction::SetTrue)
            .global(true)
            .hide(hidden)
            .help("Show the Onlyfile path"),
        Arg::new("set")
            .short('s')
            .long("set")
            .value_name("NAME=VALUE")
            .action(ArgAction::Append)
            .global(true)
            .hide(hidden)
            .help("Set a task value"),
        Arg::new("dry-run")
            .long("dry-run")
            .action(ArgAction::SetTrue)
            .global(true)
            .hide(hidden)
            .help("Show the task plan"),
        Arg::new("full")
            .long("full")
            .action(ArgAction::SetTrue)
            .requires("dry-run")
            .global(true)
            .hide(hidden)
            .help("Show the task plan with commands"),
        Arg::new("quiet")
            .short('q')
            .long("quiet")
            .action(ArgAction::SetTrue)
            .global(true)
            .hide(hidden)
            .help("Hide Only progress"),
        Arg::new("fmt")
            .long("fmt")
            .action(ArgAction::SetTrue)
            .global(true)
            .hide(hidden)
            .help("Format the Onlyfile"),
        Arg::new("format-check")
            .long("check")
            .action(ArgAction::SetTrue)
            .requires("fmt")
            .global(true)
            .hide(hidden)
            .help("Check formatting without writing"),
    ];

    for option in options {
        command = command.arg(option);
    }

    command
}

/// Builds the full dynamic CLI from a parsed semantic document.
///
/// Args:
/// document: Parsed task document.
///
/// Returns:
/// Clap command with global tasks and groups wired as subcommands.
pub fn build_cli(document: &DocumentAst) -> Command {
    let mut cmd = build_global_cli();
    let globals = global_plan_params(document);

    for task in selected_tasks(document, global_tasks(document)) {
        cmd = cmd.subcommand(build_task_command(task, &globals));
    }

    for namespace in document
        .namespaces
        .iter()
        .filter(|namespace| namespace_has_visible_tasks(document, namespace.name.as_str()))
    {
        cmd = cmd.subcommand(build_namespace_command(document, namespace, &globals));
    }

    cmd
}

/// Renders dynamic help from a parsed semantic document.
///
/// Args:
/// document: Parsed task document.
///
/// Returns:
/// Help text including dynamically discovered tasks and groups.
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
/// User-facing task list with global tasks and groups.
pub fn render_available_tasks(document: &DocumentAst) -> String {
    let globals = global_plan_params(document);
    let tasks = task_listing_entries(document, global_tasks(document), &globals);
    let namespaces = document
        .namespaces
        .iter()
        .filter(|namespace| namespace_has_visible_tasks(document, namespace.name.as_str()))
        .map(|namespace| {
            (
                namespace.name.to_string(),
                namespace_summary(namespace, &globals),
            )
        })
        .collect::<Vec<_>>();

    let name_width = tasks
        .iter()
        .chain(&namespaces)
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or_default();

    let mut sections = Vec::new();
    if !tasks.is_empty() {
        sections.push(render_listing_section(
            "Tasks",
            &tasks,
            name_width,
            TermAnsiColor::BrightCyan,
        ));
    }
    if !namespaces.is_empty() {
        sections.push(render_listing_section(
            "Groups",
            &namespaces,
            name_width,
            TermAnsiColor::BrightYellow,
        ));
    }

    if sections.is_empty() {
        "Tasks:\n".to_string()
    } else {
        sections.join("\n")
    }
}

fn render_listing_section(
    title: &str,
    entries: &[(String, String)],
    name_width: usize,
    name_color: TermAnsiColor,
) -> String {
    let header_style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightGreen.into()))
        .bold();
    let name_style = TermStyle::new().fg_color(Some(name_color.into()));
    let mut output = format!(
        "{}{title}:{}\n",
        header_style.render(),
        header_style.render_reset()
    );

    for (name, doc) in entries {
        output.push_str(&format!(
            "  {}{}{}",
            name_style.render(),
            name,
            name_style.render_reset()
        ));
        if !doc.is_empty() {
            let padding = " ".repeat(name_width.saturating_sub(name.len()) + 4);
            output.push_str(&format!("{padding}# {doc}"));
        }
        output.push('\n');
    }

    output
}

/// Renders help for a group and all of its child tasks.
///
/// Args:
/// document: Parsed task document.
/// namespace: Parsed group definition.
///
/// Returns:
/// Help text for the group subcommand.
pub fn render_namespace_help(document: &DocumentAst, namespace: &NamespaceAst) -> StyledStr {
    let globals = global_plan_params(document);
    let mut command = build_namespace_command(document, namespace, &globals);
    command.render_help()
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
        "Run '{}only --help{}' for help.",
        style.render(),
        style.render_reset()
    )
}

fn build_namespace_command(
    document: &DocumentAst,
    namespace: &NamespaceAst,
    globals: &[PlanParam],
) -> Command {
    let entries = task_listing_entries(
        document,
        namespace_tasks(document, namespace.name.as_str()),
        globals,
    );
    let name_width = entries
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or_default();
    let listing = if entries.is_empty() {
        "Tasks:\n".to_string()
    } else {
        render_listing_section("Tasks", &entries, name_width, TermAnsiColor::BrightCyan)
    };
    let command = hide_subcommand_help(
        Command::new(namespace.name.to_string())
            .bin_name(format!("only {}", namespace.name))
            .disable_help_subcommand(true)
            .styles(cli_styles())
            .override_usage(format!("only {} [COMMAND] [OPTIONS]", namespace.name))
            .help_template(NAMESPACE_HELP_TEMPLATE)
            .after_help(StyledStr::from(listing)),
    );
    let mut cmd = with_global_options(command, true);

    if let Some(about) = metadata_about(
        namespace.metadata.help.as_deref(),
        namespace.metadata.desc.as_deref(),
        globals,
    ) {
        cmd = cmd.about(about);
    }

    for task in selected_tasks(document, namespace_tasks(document, namespace.name.as_str())) {
        cmd = cmd.subcommand(build_task_command(task, globals).hide(true));
    }

    cmd
}

fn namespace_summary(namespace: &NamespaceAst, globals: &[PlanParam]) -> String {
    namespace
        .metadata
        .help
        .as_ref()
        .map(|help| render_metadata(help, globals))
        .unwrap_or_default()
}

fn build_task_command(task: &TaskAst, globals: &[PlanParam]) -> Command {
    let about = metadata_about(
        task.metadata.help.as_deref(),
        task.metadata.desc.as_deref(),
        globals,
    );
    let command = hide_subcommand_help(
        Command::new(task.name.to_string())
            .styles(cli_styles())
            .about(about.unwrap_or_default())
            .override_usage(task_usage(task))
            .help_template(if task.params.is_empty() {
                TASK_HELP_TEMPLATE
            } else {
                TASK_HELP_WITH_ARGS_TEMPLATE
            })
            .hide(task.is_helper()),
    );
    let mut cmd = with_global_options(command, true);

    for (index, param) in task.params.iter().enumerate() {
        let arg = if param.is_slice {
            Arg::new(param.name.to_string())
                .index(index + 1)
                .required(false)
                .num_args(0..)
                .trailing_var_arg(true)
                .allow_hyphen_values(true)
                .help("Slice parameter")
        } else if let Some(default) = &param.default_value {
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

fn hide_subcommand_help(command: Command) -> Command {
    command.disable_help_flag(true).arg(
        Arg::new("help")
            .short('h')
            .long("help")
            .action(ArgAction::Help)
            .hide(true),
    )
}

fn task_usage(task: &TaskAst) -> String {
    let mut path = vec!["only".to_string()];
    if let Some(namespace) = &task.namespace {
        path.push(namespace.to_string());
    }
    path.push(task.name.to_string());

    for parameter in &task.params {
        let name = parameter.name.to_uppercase();
        if parameter.is_slice {
            path.push(format!("[{name}]..."));
        } else {
            path.push(format!("[{name}]"));
        }
    }

    path.push("[OPTIONS]".to_string());

    path.join(" ")
}

fn selected_tasks<'a>(
    document: &'a DocumentAst,
    tasks: impl IntoIterator<Item = &'a TaskAst>,
) -> Vec<&'a TaskAst> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();

    for task in tasks {
        let target = task.qualified_name();
        if seen.insert(target.clone()) {
            unique.push(select_root_task_variant(document, target.as_str()).unwrap_or(task));
        }
    }

    unique
}

fn task_listing_entries<'a>(
    document: &'a DocumentAst,
    tasks: impl IntoIterator<Item = &'a TaskAst>,
    globals: &[PlanParam],
) -> Vec<(String, String)> {
    selected_tasks(document, tasks)
        .into_iter()
        .filter(|task| !task.is_helper())
        .map(|task| {
            (
                task.name.to_string(),
                task.doc
                    .as_ref()
                    .map(|text| render_metadata(text, globals))
                    .unwrap_or_default(),
            )
        })
        .collect()
}

fn global_plan_params(document: &DocumentAst) -> Vec<PlanParam> {
    let mut globals = document
        .directives
        .iter()
        .filter_map(|directive| match directive {
            only_semantic::DirectiveAst::Variable { name, value, .. } => Some(PlanParam {
                name: name.to_string(),
                default_value: Some(value.to_string()),
                value: Some(value.to_string()),
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    globals.sort_by(|left, right| left.name.cmp(&right.name));
    globals
}

fn render_metadata(text: &str, globals: &[PlanParam]) -> String {
    render_command(text, globals).unwrap_or_else(|_| text.to_string())
}

fn metadata_about(
    help: Option<&str>,
    desc: Option<&str>,
    globals: &[PlanParam],
) -> Option<StyledStr> {
    let help = help.map(|text| render_metadata(text, globals));
    let desc = desc.map(|text| render_metadata(text, globals));

    match (help, desc) {
        (Some(help), Some(desc)) => Some(StyledStr::from(format!(
            "{help}\n\n{}",
            details_text(&desc)
        ))),
        (Some(help), None) => Some(StyledStr::from(help)),
        (None, Some(desc)) => Some(StyledStr::from(details_text(&desc))),
        (None, None) => None,
    }
}

fn details_text(desc: &str) -> String {
    let label_style = TermStyle::new()
        .fg_color(Some(TermAnsiColor::BrightGreen.into()))
        .bold();
    let indented = desc.replace('\n', "\n         ");
    format!(
        "{}Details:{} {}",
        label_style.render(),
        label_style.render_reset(),
        indented
    )
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

fn namespace_has_visible_tasks(document: &DocumentAst, namespace: &str) -> bool {
    namespace_tasks(document, namespace).any(|task| !task.is_helper())
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
        build_cli, render_available_tasks, render_error_message, render_global_help, render_help,
        render_namespace_help,
    };
    use crate::parse_onlyfile;
    use clap::error::ErrorKind;
    use only_semantic::compile_document;
    use std::panic;

    #[test]
    fn renders_colored_error_message() {
        let rendered = render_error_message("task failed");
        assert!(rendered.contains("Error:"));
        assert!(rendered.contains("task failed"));
    }

    #[test]
    fn group_lists_without_slash() {
        let document = parse_onlyfile(
            "!version 0.4
group dev {
[help] Default developer workflow.
workflow():
    echo ok
}
",
        )
        .expect("document should parse");

        let mut cmd = build_cli(&document);
        let help = cmd.render_help().to_string();

        assert!(help.contains("dev"));
        assert!(!help.contains("dev/"));
    }

    #[test]
    fn group_help_alias() {
        let document = parse_onlyfile(
            "!version 0.4
group dev {
[help] Default developer workflow.
workflow():
    echo ok
}
",
        )
        .expect("document should parse");

        let matches = build_cli(&document)
            .try_get_matches_from(["only", "dev", "--help"])
            .expect_err("help should short-circuit parsing");

        assert_eq!(matches.kind(), ErrorKind::DisplayHelp);
        let help = matches.to_string();
        assert!(help.contains("Tasks:"));
        assert!(help.contains("Default developer workflow."));
        assert!(!help.contains("Commands:"));
    }

    #[test]
    fn shows_task_desc() {
        let document = compile_document(
            "[help] Deploy app\n[desc] Supports staging and production\ndeploy():\n    true\n",
        )
        .document;
        let listing = render_available_tasks(&document);
        assert!(listing.contains("# Deploy app"));
        assert!(!listing.contains("Supports staging"));

        for flag in ["-h", "--help"] {
            let error = build_cli(&document)
                .try_get_matches_from(["only", "deploy", flag])
                .expect_err("task help should short-circuit parsing");
            assert_eq!(error.kind(), ErrorKind::DisplayHelp);
            let help = error.to_string();
            assert!(help.contains("Deploy app"));
            assert!(help.contains("Supports staging and production"));
        }
    }

    #[test]
    fn separates_task_options() {
        let document = compile_document(
            "[help] Install only\n[desc] Install to ~/.local/bin/only.\ninstall():\n    true\n",
        )
        .document;
        let error = build_cli(&document)
            .try_get_matches_from(["only", "install", "-h"])
            .expect_err("task help should short-circuit parsing");
        let help = error.to_string();

        assert!(help.contains("Install only\n\n"));
        assert!(help.contains("Details: Install to ~/.local/bin/only."));
        assert!(help.contains("only install [OPTIONS]"));
        assert!(!help.contains("Install only\n\nInstall to"));
        assert!(!help.contains("Options:"));
        assert!(help.contains("Run `only -h` to see available options."));
        assert!(!help.contains("--path"));
        assert!(!help.contains("--dry-run"));

        let root_help = render_help(&document).to_string();
        assert!(root_help.contains("--path"));
        assert!(root_help.contains("--dry-run"));
    }

    #[test]
    fn task_help_styles() {
        let document = compile_document(
            "[help] Install only\n[desc] Install to ~/.local/bin/only.\ninstall():\n    true\n",
        )
        .document;
        let mut command = build_cli(&document);
        let task = command
            .find_subcommand_mut("install")
            .expect("task should be present");
        let help = task.render_help().ansi().to_string();

        assert!(help.contains("\u{1b}[1m\u{1b}[92mUsage:"));
        assert!(help.contains("\u{1b}[1m\u{1b}[92mDetails:"));
        assert!(!help.contains("Options:"));
        assert!(help.contains("only install [OPTIONS]"));
        assert!(!help.contains("--path"));
    }

    #[test]
    fn task_help_arguments() {
        let document =
            compile_document("[help] Build only\nbuild(profile=\"debug\", args..):\n    true\n")
                .document;
        let error = build_cli(&document)
            .try_get_matches_from(["only", "build", "-h"])
            .expect_err("task help should short-circuit parsing");
        let help = error.to_string();

        assert!(help.contains("Usage: only build [PROFILE] [ARGS]... [OPTIONS]"));
        assert!(help.contains("Arguments:"));
        assert!(help.contains("[PROFILE]"));
        assert!(help.contains("[ARGS]..."));
        assert!(!help.contains("--path"));
    }

    #[test]
    fn accepts_task_options() {
        let document = compile_document("[help] Check project\ncheck():\n    true\n").document;
        let matches = build_cli(&document)
            .try_get_matches_from(["only", "check", "--dry-run"])
            .expect("global option should parse after task");

        assert!(matches.get_flag("dry-run"));
        assert_eq!(matches.subcommand_name(), Some("check"));
    }

    #[test]
    fn renders_global_help() {
        let help = render_global_help().to_string();

        assert!(help.contains("Usage: only [TASK] [ARGS]... [OPTIONS]"));
        assert!(help.contains("--path"));
        assert!(help.contains("--update"));
        assert!(help.contains("--upgrade"));
        assert!(help.contains("--where"));
        assert!(help.contains("--version"));
        assert!(help.contains("Repo: https://github.com/KercyDing/only"));
    }

    #[test]
    fn dynamic_help_lists_tasks() {
        let document = parse_onlyfile(
            "!version 0.4
[help] Run tests.
test():
    cargo test

group dev {
[help] Default developer workflow.
workflow():
    echo ok
}
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
    fn renders_group_help() {
        let document = parse_onlyfile(
            "!version 0.4
group dev {
[help] Default developer workflow.
workflow():
    echo ok

[help] Run a group smoke command.
smoke():
    echo smoke
}
",
        )
        .expect("document should parse");

        let help = render_namespace_help(&document, &document.namespaces[0]).to_string();
        assert!(help.contains("Usage:"));
        assert!(help.contains("[OPTIONS]"));
        assert!(help.contains("dev [COMMAND]"));
        assert!(help.contains("Tasks:"));
        assert!(help.contains("workflow"));
        assert!(help.contains("Default developer workflow."));
        assert!(help.contains("smoke"));
        assert!(help.contains("Run a group smoke command."));
        assert!(!help.contains("Commands:"));
    }

    #[test]
    fn shows_group_metadata() {
        let document = compile_document(
            "!version 0.4\n!var mode = \"development\"\n[help] Dev builds\n[desc] Build in {{mode}} mode.\ngroup dev {\n    [help] Build project\n    build():\n        cargo build\n}\n",
        )
        .document;

        let listing = render_available_tasks(&document);
        assert!(listing.contains("# Dev builds"));
        assert!(!listing.contains("Build in development mode."));

        let help = render_namespace_help(&document, &document.namespaces[0]).to_string();
        assert!(help.contains("Dev builds"));
        assert!(help.contains("Build in development mode."));
    }

    #[test]
    fn available_tasks_listing() {
        let document = parse_onlyfile(
            "!version 0.4
[help] Run tests.
test():
    cargo test

group dev {
[help] Default developer workflow.
workflow():
    echo ok
}
",
        )
        .expect("document should parse");

        let listing = render_available_tasks(&document);
        assert!(listing.contains("Tasks:"));
        assert!(listing.contains("Groups:"));
        assert!(listing.contains("test"));
        assert!(listing.contains("# Run tests."));
        assert!(listing.contains("dev"));
        assert!(!listing.contains("[group]"));
        assert!(!listing.contains("Default developer workflow."));
        assert!(listing.contains("\u{1b}[1m\u{1b}[92mTasks:"));
        assert!(listing.contains("\u{1b}[1m\u{1b}[92mGroups:"));
        assert!(!listing.contains("\u{1b}[1m\u{1b}[96mtest"));
        assert!(!listing.contains("\u{1b}[1m\u{1b}[93mdev"));
    }

    #[test]
    fn selects_variant_help() {
        let document = parse_onlyfile(
            "!version 0.4\n[help] Guarded tests\n[desc] Uses another runner.\ntest() ? @os(\"not-a-real-os\"):\n    true\n\n[help] Cargo tests\n[desc] Uses Cargo.\ntest():\n    true\n",
        )
        .expect("document should parse");

        let listing = render_available_tasks(&document);
        assert!(listing.contains("# Cargo tests"));
        assert!(!listing.contains("# Guarded tests"));

        let error = build_cli(&document)
            .try_get_matches_from(["only", "test", "--help"])
            .expect_err("task help should short-circuit parsing");
        let help = error.to_string();
        assert!(help.contains("Cargo tests"));
        assert!(help.contains("Uses Cargo."));
        assert!(!help.contains("Uses another runner."));
    }

    #[test]
    fn inherits_variant_help() {
        let document = parse_onlyfile(
            "!version 0.4\n[help] Run tests\n[desc] Run the project tests.\ntest() ? @os(\"not-a-real-os\"):\n    true\n\ntest():\n    true\n",
        )
        .expect("document should parse");

        let listing = render_available_tasks(&document);
        assert!(listing.contains("# Run tests"));

        let error = build_cli(&document)
            .try_get_matches_from(["only", "test", "--help"])
            .expect_err("task help should short-circuit parsing");
        let help = error.to_string();
        assert!(help.contains("Run tests"));
        assert!(help.contains("Run the project tests."));
    }

    #[test]
    fn group_summary() {
        let document = parse_onlyfile(
            "!version 0.4\n[help] Developer workflow.\ngroup dev {\n    [help] Run smoke.\n    smoke():\n        echo smoke\n}\n",
        )
        .expect("document should parse");

        let listing = render_available_tasks(&document);
        assert!(listing.contains("Developer workflow."));
        assert!(!listing.contains("Run smoke."));
    }

    #[test]
    fn group_help_without_doc() {
        let document = parse_onlyfile(
            "!version 0.4
group dev {
    [help] Run smoke.
    smoke():
        echo smoke
}
",
        )
        .expect("document should parse");

        let help = render_namespace_help(&document, &document.namespaces[0]).to_string();
        assert!(help.starts_with("Usage:"));
        assert!(help.contains("[OPTIONS]"));
        assert!(help.contains("dev [COMMAND]"));
    }

    #[test]
    fn group_about() {
        let document = parse_onlyfile(
            "!version 0.4
[help] Developer workflow.
group dev {
    [help] Run smoke.
    smoke():
        echo smoke
}
",
        )
        .expect("document should parse");

        let help = render_namespace_help(&document, &document.namespaces[0]).to_string();
        assert!(help.contains("Developer workflow."));
    }

    #[test]
    fn hides_helpers() {
        let document = parse_onlyfile(
            "!version 0.4\n# Internal test helper.\n_test_helper():\n    cargo test\ntest():\n    cargo test\n\ngroup dev {\n    _workflow():\n        echo hidden\n    workflow():\n        echo ok\n}\n",
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
    fn hides_helper_group() {
        let document = parse_onlyfile(
            "!version 0.4\ncheck():\n    cargo check\n\n[help] Hidden group.\ngroup dev {\n    _hidden():\n        echo hidden\n}\n",
        )
        .expect("document should parse");

        let listing = render_available_tasks(&document);
        assert!(listing.contains("check"));
        assert!(!listing.contains("dev"));
        assert!(!listing.contains("Hidden group."));

        let namespace_help = render_namespace_help(&document, &document.namespaces[0]).to_string();
        assert!(namespace_help.contains("Hidden group."));
        assert!(namespace_help.contains("Usage:"));
        assert!(namespace_help.contains("[OPTIONS]"));
        assert!(namespace_help.contains("dev"));
        assert!(namespace_help.contains("Tasks:"));
        assert!(!namespace_help.contains("Options:"));
        assert!(!namespace_help.contains("Commands:"));
        assert!(!namespace_help.contains("_hidden"));
    }

    #[test]
    fn guarded_variants_no_panic() {
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
