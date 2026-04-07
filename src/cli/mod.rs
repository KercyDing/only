pub mod app;
pub mod args;
pub mod dispatch;

pub use args::CliInput;

pub fn parse() -> crate::diagnostic::error::Result<CliInput> {
    CliInput::from_matches(app::build().get_matches())
}
