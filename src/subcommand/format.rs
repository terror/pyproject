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
        let display_path = self.path.display().to_string();

        let diff = TextDiff::from_lines(&content, &formatted)
          .unified_diff()
          .context_radius(3)
          .header(&display_path, &format!("{display_path} (formatted)"))
          .to_string();

        let colored_diff = diff
          .lines()
          .map(|line| match line.chars().next() {
            Some('+') => line.green().to_string(),
            Some('-') => line.red().to_string(),
            Some('@') => line.blue().to_string(),
            Some(' ') => line.dimmed().to_string(),
            Some('\\') => line.yellow().to_string(),
            _ => line.to_string(),
          })
          .collect::<Vec<_>>()
          .join("\n");

        println!("{colored_diff}");

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
