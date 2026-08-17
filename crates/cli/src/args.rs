use crate::error::{OnlyError, Result};
use clap::ArgMatches;
use only_semantic::{DocumentAst, NamespaceAst, TaskAst};
use std::ffi::OsString;
use std::path::PathBuf;

/// Normalized CLI input shared across discovery, planning, and runtime phases.
///
/// Args:
/// None.
///
/// Returns:
/// Parsed top-level flags, task target path, and parameter overrides.
///
/// Edge Cases:
/// `task_path` stays empty after phase-one parsing and is filled during phase two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliInput {
    pub onlyfile_path: Option<PathBuf>,
    pub print_discovered_path: bool,
    pub dry_run: bool,
    pub dry_run_full: bool,
    pub quiet: bool,
    pub top_level_help_requested: bool,
    pub top_level_version_requested: bool,
    pub top_level_upgrade_requested: bool,
    pub format_requested: bool,
    pub format_check: bool,
    pub task_path: Vec<String>,
    pub parameter_overrides: Vec<(String, String)>,
}

impl CliInput {
    /// Builds normalized CLI input from clap matches.
    ///
    /// Args:
    /// matches: Parsed clap matches.
    ///
    /// Returns:
    /// Normalized CLI input or an error for invalid override syntax.
    pub fn from_matches(matches: ArgMatches) -> Result<Self> {
        let parameter_overrides = matches
            .get_many::<String>("set")
            .into_iter()
            .flatten()
            .map(|item| parse_override(item))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            onlyfile_path: matches.get_one::<String>("onlyfile").map(PathBuf::from),
            print_discovered_path: matches.get_flag("print-path"),
            dry_run: matches.get_flag("dry-run"),
            dry_run_full: matches.get_flag("full"),
            quiet: matches.get_flag("quiet"),
            top_level_help_requested: false,
            top_level_version_requested: false,
            top_level_upgrade_requested: matches.get_flag("upgrade") || matches.get_flag("update"),
            format_requested: matches.get_flag("fmt"),
            format_check: matches.get_flag("format-check"),
            task_path: vec![],
            parameter_overrides,
        })
    }

    /// Extracts task path from subcommand chain.
    ///
    /// Args:
    /// matches: Parsed clap matches with subcommands.
    /// document: Parsed semantic document used to resolve task parameters.
    ///
    /// Returns:
    /// Self with task path populated.
    pub fn with_task_path(mut self, matches: ArgMatches, document: &DocumentAst) -> Self {
        let mut path = Vec::new();
        let mut current = matches;

        while let Some((name, sub_matches)) = current.subcommand() {
            path.push(name.trim_end_matches('/').to_string());
            current = sub_matches.clone();
        }

        if let Some(task) = task_for_path(document, &path) {
            for parameter in &task.params {
                if parameter.is_slice {
                    if let Some(values) = current.get_many::<String>(parameter.name.as_str()) {
                        path.extend(values.cloned());
                    }
                    continue;
                }

                if let Some(value) = current.get_one::<String>(parameter.name.as_str()) {
                    path.push(value.clone());
                }
            }
        }

        self.task_path = path;
        self
    }
}

/// Extracts global CLI options from raw argv without consuming task segments.
///
/// Args:
/// args: Full process argv, including binary name.
///
/// Returns:
/// Partial CLI input containing only global options needed before `Onlyfile` discovery.
///
/// Edge Cases:
/// Stops parsing global options after `--` and ignores `-h` / `--help` so phase two can render
/// dynamic task help.
pub fn parse_global_options() -> Result<CliInput> {
    parse_global_options_from(std::env::args_os())
}

/// Parses the full dynamic CLI after `Onlyfile` loading succeeds.
///
/// Args:
/// document: Parsed task document used to build dynamic subcommands.
///
/// Returns:
/// Normalized CLI input with resolved task path.
pub fn parse_with_onlyfile(document: &DocumentAst) -> Result<CliInput> {
    let matches = crate::render::build_cli(document).get_matches();
    let input = CliInput::from_matches(matches.clone())?.with_task_path(matches, document);
    Ok(input)
}

pub(crate) fn parse_global_options_from<I, T>(args: I) -> Result<CliInput>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut onlyfile_path = None;
    let mut print_discovered_path = false;
    let mut dry_run = false;
    let mut dry_run_full = false;
    let mut quiet = false;
    let mut top_level_help_requested = false;
    let mut top_level_version_requested = false;
    let mut top_level_upgrade_requested = false;
    let mut format_requested = false;
    let mut format_check = false;
    let mut parameter_overrides = Vec::new();
    let mut seen_task_token = false;
    let mut iter = args.into_iter().map(Into::into);

    let _ = iter.next();

    while let Some(arg) = iter.next() {
        if arg == "--" {
            break;
        }

        let Some(text) = arg.to_str() else {
            continue;
        };

        match text {
            "-f" | "--file" => {
                let value = iter
                    .next()
                    .ok_or_else(|| OnlyError::parse(format!("option '{text}' needs a value")))?;
                onlyfile_path = Some(PathBuf::from(value));
            }
            "-p" | "--path" => {
                print_discovered_path = true;
            }
            "--dry-run" => {
                dry_run = true;
            }
            "--full" => {
                dry_run_full = true;
            }
            "-q" | "--quiet" => {
                quiet = true;
            }
            "-s" | "--set" => {
                let value = iter
                    .next()
                    .ok_or_else(|| OnlyError::parse(format!("option '{text}' needs a value")))?;
                parameter_overrides.push(parse_override(&os_string_to_string(value, text)?)?);
            }
            "-h" | "--help" => {
                if !seen_task_token {
                    top_level_help_requested = true;
                }
            }
            "-V" | "--version" => {
                if !seen_task_token {
                    top_level_version_requested = true;
                }
            }
            "--upgrade" | "--update" => {
                if !seen_task_token {
                    top_level_upgrade_requested = true;
                }
            }
            "--fmt" => format_requested = true,
            "--check" => format_check = true,
            _ => {
                if let Some(value) = text.strip_prefix("--file=") {
                    onlyfile_path = Some(PathBuf::from(value));
                } else if let Some(value) = text.strip_prefix("--set=") {
                    parameter_overrides.push(parse_override(value)?);
                } else if let Some(value) = text.strip_prefix("-s") {
                    if !value.is_empty() {
                        parameter_overrides.push(parse_override(value)?);
                    }
                } else if let Some(value) = text.strip_prefix("-f") {
                    if !value.is_empty() {
                        onlyfile_path = Some(PathBuf::from(value));
                    }
                } else if !text.starts_with('-') {
                    seen_task_token = true;
                }
            }
        }
    }

    Ok(CliInput {
        onlyfile_path,
        print_discovered_path,
        dry_run,
        dry_run_full,
        quiet,
        top_level_help_requested,
        top_level_version_requested,
        top_level_upgrade_requested,
        format_requested,
        format_check,
        task_path: vec![],
        parameter_overrides,
    })
}

fn parse_override(item: &str) -> Result<(String, String)> {
    let Some((name, value)) = item.split_once('=') else {
        return Err(OnlyError::parse(format!(
            "invalid value '{item}' for --set\nhelp: use NAME=VALUE"
        )));
    };

    let name = name.trim();
    if name.is_empty() {
        return Err(OnlyError::parse(format!(
            "parameter name is empty in '{item}'"
        )));
    }

    Ok((name.to_owned(), value.to_owned()))
}

fn os_string_to_string(value: OsString, option: &str) -> Result<String> {
    value
        .into_string()
        .map_err(|_| OnlyError::parse(format!("option '{option}' needs UTF-8 text")))
}

fn task_for_path<'a>(document: &'a DocumentAst, path: &[String]) -> Option<&'a TaskAst> {
    match path {
        [task] => document
            .tasks
            .iter()
            .find(|item| item.namespace.is_none() && item.name == task.as_str()),
        [namespace, task, ..] => namespace_for_name(document, namespace).and_then(|scope| {
            document.tasks.iter().find(|item| {
                item.namespace.as_deref() == Some(scope.name.as_str()) && item.name == task.as_str()
            })
        }),
        _ => None,
    }
}

fn namespace_for_name<'a>(document: &'a DocumentAst, name: &str) -> Option<&'a NamespaceAst> {
    document
        .namespaces
        .iter()
        .find(|namespace| namespace.name == name)
}

#[cfg(test)]
mod tests {
    use super::parse_global_options_from;
    use std::path::PathBuf;

    #[test]
    fn keeps_task_target_available_for_phase_two() {
        let cli =
            parse_global_options_from(["only", "test"]).expect("phase-one parsing should succeed");

        assert_eq!(cli.task_path, Vec::<String>::new());
        assert_eq!(cli.parameter_overrides, Vec::<(String, String)>::new());
        assert!(!cli.print_discovered_path);
        assert!(!cli.top_level_help_requested);
        assert!(!cli.top_level_version_requested);
        assert!(cli.onlyfile_path.is_none());
    }

    #[test]
    fn collects_global_options_without_consuming_task_segments() {
        let cli = parse_global_options_from([
            "only",
            "frontend",
            "build",
            "-s",
            "profile=prod",
            "--path",
            "-fOnlyfile.dev",
        ])
        .expect("phase-one parsing should succeed");

        assert_eq!(cli.onlyfile_path.unwrap(), PathBuf::from("Onlyfile.dev"));
        assert!(cli.print_discovered_path);
        assert!(!cli.top_level_help_requested);
        assert_eq!(
            cli.parameter_overrides,
            vec![("profile".into(), "prod".into())]
        );
    }

    #[test]
    fn collects_attached_short_parameter_override() {
        let cli = parse_global_options_from(["only", "-sprofile=prod", "build"])
            .expect("phase-one parsing should succeed");

        assert_eq!(
            cli.parameter_overrides,
            vec![("profile".into(), "prod".into())]
        );
    }

    #[test]
    fn records_top_level_help_requests() {
        let cli = parse_global_options_from(["only", "--help"])
            .expect("phase-one parsing should succeed");

        assert!(cli.top_level_help_requested);
    }

    #[test]
    fn records_top_level_version_requests() {
        let cli = parse_global_options_from(["only", "--version"])
            .expect("phase-one parsing should succeed");

        assert!(cli.top_level_version_requested);
    }

    #[test]
    fn records_top_level_upgrade_requests() {
        let cli = parse_global_options_from(["only", "--upgrade"])
            .expect("phase-one parsing should succeed");

        assert!(cli.top_level_upgrade_requested);
    }

    #[test]
    fn records_top_level_update_requests() {
        let cli = parse_global_options_from(["only", "--update"])
            .expect("phase-one parsing should succeed");

        assert!(cli.top_level_upgrade_requested);
    }

    #[test]
    fn ignores_nested_help_requests_after_task_token() {
        let cli = parse_global_options_from(["only", "dev", "--help"])
            .expect("phase-one parsing should succeed");

        assert!(!cli.top_level_help_requested);
    }

    #[test]
    fn ignores_nested_version_requests_after_task_token() {
        let cli = parse_global_options_from(["only", "dev", "--version"])
            .expect("phase-one parsing should succeed");

        assert!(!cli.top_level_version_requested);
    }

    #[test]
    fn ignores_nested_upgrade_requests_after_task_token() {
        let cli = parse_global_options_from(["only", "dev", "--upgrade"])
            .expect("phase-one parsing should succeed");

        assert!(!cli.top_level_upgrade_requested);
    }

    #[test]
    fn stops_collecting_globals_after_separator() {
        let cli =
            parse_global_options_from(["only", "run", "--", "--path", "--set", "profile=prod"])
                .expect("phase-one parsing should succeed");

        assert!(!cli.print_discovered_path);
        assert!(!cli.top_level_help_requested);
        assert!(!cli.top_level_version_requested);
        assert!(!cli.top_level_upgrade_requested);
        assert!(cli.parameter_overrides.is_empty());
    }
}
