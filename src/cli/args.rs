use std::path::PathBuf;

use clap::ArgMatches;

use crate::diagnostic::error::{OnlyError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliInput {
    pub onlyfile_path: Option<PathBuf>,
    pub print_discovered_path: bool,
    pub positionals: Vec<String>,
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
            print_discovered_path: matches.get_flag("print-discovered-path"),
            positionals: matches
                .get_many::<String>("positionals")
                .into_iter()
                .flatten()
                .cloned()
                .collect(),
            parameter_overrides,
        })
    }
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

#[cfg(test)]
mod tests {
    use crate::cli::{CliInput, app};

    #[test]
    fn rejects_invalid_override_syntax() {
        let matches = app::build()
            .try_get_matches_from(["only", "task", "--set", "broken"])
            .expect("clap should parse raw argument shape");

        let error = CliInput::from_matches(matches).expect_err("invalid override should fail");
        assert_eq!(
            error.to_string(),
            "invalid parameter override 'broken'; expected NAME=VALUE"
        );
    }

    #[test]
    fn accepts_valid_override_syntax() {
        let matches = app::build()
            .try_get_matches_from(["only", "task", "--set", "name=value"])
            .expect("clap should parse valid override");

        let cli = CliInput::from_matches(matches).expect("override should normalize");
        assert_eq!(cli.positionals, vec!["task".to_owned()]);
        assert_eq!(
            cli.parameter_overrides,
            vec![("name".into(), "value".into())]
        );
    }
}
