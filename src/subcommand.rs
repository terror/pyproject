use {super::*, check::Check, format::Format};

mod check;
mod format;
mod server;

#[derive(Debug, Parser)]
pub(crate) enum Subcommand {
  #[command(
    about = "Check a pyproject.toml file for errors and warnings",
    visible_alias = "lint"
  )]
  Check(Check),
  #[command(about = "Format a pyproject.toml file", visible_alias = "fmt")]
  Format(Format),
  #[command(about = "Start the language server", visible_alias = "lsp")]
  Server,
}

impl Subcommand {
  pub(crate) async fn run(self) -> Result {
    match self {
      Self::Check(check) => check.run(),
      Self::Format(format) => format.run(),
      Self::Server => server::run().await,
    }
  }
}
