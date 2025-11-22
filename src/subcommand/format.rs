use super::*;

#[derive(Debug, Parser)]
pub(crate) struct Format {
  #[arg(long, conflicts_with = "write")]
  check: bool,
  #[arg(value_name = "PATH")]
  path: PathBuf,
  #[arg(long, conflicts_with = "check")]
  write: bool,
}

impl Format {
  pub(crate) fn run(self) -> Result<()> {
    let content = fs::read_to_string(&self.path)?;

    let formatted =
      taplo::formatter::format(&content, taplo::formatter::Options::default());

    if self.check {
      if formatted != content {
        println!("{}", self.path.display());
        process::exit(1);
      }

      return Ok(());
    }

    if self.write {
      if formatted != content {
        fs::write(&self.path, formatted)?;
      }

      return Ok(());
    }

    print!("{formatted}");

    Ok(())
  }
}
