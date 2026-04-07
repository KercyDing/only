use std::ffi::OsString;
use std::path::PathBuf;

use clap::ArgMatches;

use crate::diagnostic::error::{OnlyError, Result};
use crate::model::{Namespace, Onlyfile, TaskDefinition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliInput {
    pub onlyfile_path: Option<PathBuf>,
    pub print_discovered_path: bool,
    pub top_level_help_requested: bool,
    pub top_level_version_requested: bool,
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
            top_level_help_requested: false,
            top_level_version_requested: false,
            task_path: vec![],
            parameter_overrides,
        })
    }

    /// Extracts task path from subcommand chain.
    ///
    /// Args:
    /// matches: Parsed clap matches with subcommands.
    ///
    /// Returns:
    /// Self with task_path populated.
    pub fn with_task_path(mut self, matches: ArgMatches, onlyfile: &Onlyfile) -> Self {
        let mut path = Vec::new();
        let mut current = matches;

        while let Some((name, sub_matches)) = current.subcommand() {
            path.push(name.trim_end_matches('/').to_string());
            current = sub_matches.clone();
        }

        if let Some(task) = task_for_path(onlyfile, &path) {
            for parameter in &task.signature.parameters {
                if let Some(value) = current.get_one::<String>(&parameter.name) {
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
pub(crate) fn parse_global_options_from<I, T>(args: I) -> Result<CliInput>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut onlyfile_path = None;
    let mut print_discovered_path = false;
    let mut top_level_help_requested = false;
    let mut top_level_version_requested = false;
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
                let value = iter.next().ok_or_else(|| {
                    OnlyError::parse(format!("missing value for global option '{text}'"))
                })?;
                onlyfile_path = Some(PathBuf::from(value));
            }
            "-p" | "--path" => {
                print_discovered_path = true;
            }
            "--set" => {
                let value = iter
                    .next()
                    .ok_or_else(|| OnlyError::parse("missing value for global option '--set'"))?;
                parameter_overrides.push(parse_override(&os_string_to_string(value, "--set")?)?);
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
            _ => {
                if let Some(value) = text.strip_prefix("--file=") {
                    onlyfile_path = Some(PathBuf::from(value));
                } else if let Some(value) = text.strip_prefix("--set=") {
                    parameter_overrides.push(parse_override(value)?);
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
        top_level_help_requested,
        top_level_version_requested,
        task_path: vec![],
        parameter_overrides,
    })
}

fn parse_override(item: &str) -> Result<(String, String)> {
    let Some((name, value)) = item.split_once('=') else {
        return Err(OnlyError::parse(format!(
            "invalid parameter override '{item}'; expected NAME=VALUE"
        )));
    };

    let name = name.trim();
    if name.is_empty() {
        return Err(OnlyError::parse(format!(
            "invalid parameter override '{item}'; parameter name cannot be empty"
        )));
    }

    Ok((name.to_owned(), value.to_owned()))
}

fn os_string_to_string(value: OsString, option: &str) -> Result<String> {
    value
        .into_string()
        .map_err(|_| OnlyError::parse(format!("non-UTF-8 values are not supported for '{option}'")))
}

fn task_for_path<'a>(onlyfile: &'a Onlyfile, path: &[String]) -> Option<&'a TaskDefinition> {
    match path {
        [task] => onlyfile
            .global_tasks
            .iter()
            .find(|item| item.signature.name == *task),
        [namespace, task, ..] => namespace_for_name(onlyfile, namespace)
            .and_then(|scope| scope.tasks.iter().find(|item| item.signature.name == *task)),
        _ => None,
    }
}

fn namespace_for_name<'a>(onlyfile: &'a Onlyfile, name: &str) -> Option<&'a Namespace> {
    onlyfile
        .namespaces
        .iter()
        .find(|namespace| namespace.name == name)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::parse_global_options_from;

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
            "--set",
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
    fn stops_collecting_globals_after_separator() {
        let cli =
            parse_global_options_from(["only", "run", "--", "--path", "--set", "profile=prod"])
                .expect("phase-one parsing should succeed");

        assert!(!cli.print_discovered_path);
        assert!(!cli.top_level_help_requested);
        assert!(!cli.top_level_version_requested);
        assert!(cli.parameter_overrides.is_empty());
    }
}
