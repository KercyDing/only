pub mod app;
pub mod args;
pub mod dispatch;

pub use args::CliInput;

pub fn parse() -> CliInput {
    app::build().get_matches().into()
}
