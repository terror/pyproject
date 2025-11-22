use {super::*, check::Check, format::Format};

mod check;
mod format;
mod server;

#[derive(Debug, Parser)]
pub(crate) enum Subcommand {
  Check(Check),
  Format(Format),
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
