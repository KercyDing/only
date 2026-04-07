pub mod app;
pub mod args;

pub use args::CliInput;

use crate::diagnostic::error::Result;
use crate::model::Onlyfile;

pub fn parse_global_options() -> Result<CliInput> {
    args::parse_global_options_from(std::env::args_os())
}

pub fn parse_with_onlyfile(onlyfile: &Onlyfile) -> Result<CliInput> {
    let matches = app::build(onlyfile).get_matches();
    let input = CliInput::from_matches(matches.clone())?.with_task_path(matches);
    Ok(input)
}
