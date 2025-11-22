use {super::*, check::Check};

mod check;
mod server;

#[derive(Debug, Parser)]
pub(crate) enum Subcommand {
  Check(Check),
  Server,
}

impl Subcommand {
  pub(crate) async fn run(self) -> Result {
    match self {
      Self::Check(check) => check.run(),
      Self::Server => server::run().await,
    }
  }
}
