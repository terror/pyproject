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
  fn find_pyproject_toml() -> Result<PathBuf> {
    let mut current_dir = env::current_dir()?;

    loop {
      let candidate = current_dir.join("pyproject.toml");

      if candidate.exists() {
        return Ok(candidate);
      }

      if !current_dir.pop() {
        bail!(
          "could not find `pyproject.toml` in current directory or any parent directory"
        );
      }
    }
  }

  pub(crate) async fn run(self) -> Result {
    match self {
      Self::Check(check) => check.run(),
      Self::Format(format) => format.run(),
      Self::Server => server::run().await,
    }
  }
}
