use super::*;

mod server;

#[derive(Debug, Parser)]
pub(crate) enum Subcommand {
  Server,
}

impl Subcommand {
  pub(crate) async fn run(self) -> Result {
    match self {
      Self::Server => server::run().await,
    }
  }
}
