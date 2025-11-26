use super::*;

#[derive(Debug, Parser)]
#[command(
  name = "pyproject",
  version,
  about = "A linter and language server for pyproject.toml files",
  arg_required_else_help = true,
  disable_help_subcommand = true,
  propagate_version = true,
  help_template = "{bin} {version}\n\n{usage-heading} {usage}\n\n{all-args}{after-help}"
)]
pub(crate) struct Arguments {
  #[clap(subcommand)]
  subcommand: Subcommand,
}

impl Arguments {
  pub(crate) async fn run(self) -> Result {
    self.subcommand.run().await
  }
}
