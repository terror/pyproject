use {
  anyhow::Error,
  executable_path::executable_path,
  indoc::indoc,
  pretty_assertions::assert_eq,
  std::{fs, iter::once, path::PathBuf, process::Command, str},
  tempfile::TempDir,
};

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
struct Test<'a> {
  arguments: Vec<String>,
  directory: Option<String>,
  expected_status: i32,
  expected_stderr: String,
  expected_stdout: String,
  files: Vec<(&'a str, &'a str)>,
  tempdir: TempDir,
}

impl<'a> Test<'a> {
  fn argument(self, argument: &str) -> Self {
    Self {
      arguments: self
        .arguments
        .into_iter()
        .chain(once(argument.to_owned()))
        .collect(),
      ..self
    }
  }

  fn command(&self) -> Command {
    let mut command = Command::new(executable_path(env!("CARGO_PKG_NAME")));

    command
      .arg("check")
      .env("NO_COLOR", "1")
      .env("RUST_BACKTRACE", "0")
      .current_dir(self.current_dir());

    command.args(&self.arguments);

    command
  }

  fn current_dir(&self) -> PathBuf {
    if let Some(directory) = &self.directory {
      self.tempdir.path().join(directory)
    } else {
      self.tempdir.path().to_path_buf()
    }
  }

  fn directory(self, directory: &str) -> Self {
    Self {
      directory: Some(directory.to_owned()),
      ..self
    }
  }

  fn expected_status(self, expected_status: i32) -> Self {
    Self {
      expected_status,
      ..self
    }
  }

  fn expected_stderr(self, expected_stderr: &str) -> Self {
    Self {
      expected_stderr: expected_stderr.to_owned(),
      ..self
    }
  }

  fn expected_stdout(self, expected_stdout: &str) -> Self {
    Self {
      expected_stdout: expected_stdout.to_owned(),
      ..self
    }
  }

  fn file(self, path: &'a str, content: &'a str) -> Self {
    Self {
      files: self
        .files
        .into_iter()
        .chain(once((path, content)))
        .collect(),
      ..self
    }
  }

  fn new() -> Result<Self> {
    Ok(Self {
      arguments: Vec::new(),
      directory: None,
      expected_status: 0,
      expected_stderr: String::new(),
      expected_stdout: String::new(),
      files: Vec::new(),
      tempdir: TempDir::with_prefix("pyproject-test")?,
    })
  }

  fn normalize(&self, text: &str) -> Result<String> {
    let mut normalized = text
      .lines()
      .map(str::trim_end)
      .collect::<Vec<_>>()
      .join("\n");

    if text.ends_with('\n') {
      normalized.push('\n');
    }

    let root = self.tempdir.path().display().to_string().replace('\\', "/");

    let canonical_root = fs::canonicalize(self.tempdir.path())?
      .display()
      .to_string()
      .replace('\\', "/");

    Ok(
      normalized
        .replace('\\', "/")
        .replace(&canonical_root, "[ROOT]")
        .replace(&root, "[ROOT]"),
    )
  }

  fn run(self) -> Result {
    for (path, content) in &self.files {
      let path = self.tempdir.path().join(path);

      if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
      }

      fs::write(path, content)?;
    }

    fs::create_dir_all(self.current_dir())?;

    let output = self.command().output()?;
    let stderr = self.normalize(str::from_utf8(&output.stderr)?)?;

    assert_eq!(
      output.status.code(),
      Some(self.expected_status),
      "unexpected exit status\nstderr: {stderr}"
    );

    assert_eq!(stderr, self.expected_stderr);

    let stdout = self.normalize(str::from_utf8(&output.stdout)?)?;

    assert_eq!(stdout, self.expected_stdout);

    Ok(())
  }
}

#[test]
fn check_accepts_absolute_pyproject_path() -> Result {
  let test = Test::new()?;

  let path = test
    .tempdir
    .path()
    .join("pyproject.toml")
    .display()
    .to_string();

  test
    .file(
      "pyproject.toml",
      indoc! {
        r#"
        [project]
        name = "foo"
        version = "1.0.0"
        "#
      },
    )
    .argument(&path)
    .run()
}

#[test]
fn check_errors_when_pyproject_cannot_be_found() -> Result {
  Test::new()?
    .expected_status(1)
    .expected_stderr(
      "error: could not find `pyproject.toml` in current directory or any parent directory\n",
    )
    .run()
}

#[test]
fn check_finds_pyproject_in_parent_directory() -> Result {
  Test::new()?
    .file(
      "pyproject.toml",
      indoc! {
        r#"
        [project]
        name = "foo"
        version = "1.0.0"
        "#
      },
    )
    .directory("foo/bar")
    .run()
}

#[test]
fn check_reports_errors_and_fails() -> Result {
  Test::new()?
    .file(
      "pyproject.toml",
      indoc! {
        r#"
        [project]
        name = "Foo_Bar"
        version = "1.0.0"
        "#
      },
    )
    .argument("pyproject.toml")
    .expected_status(1)
    .expected_stdout(indoc! {
      r#"
      error[project-name]: invalid value for `project.name`
         ╭─[ pyproject.toml:2:8 ]
         │
       2 │ name = "Foo_Bar"
         │        ────┬────
         │            ╰────── `project.name` must be PEP 503 normalized (use `foo-bar`)
      ───╯
      "#
    })
    .run()
}

#[test]
fn check_reports_warnings_without_failing() -> Result {
  Test::new()?
    .file(
      "pyproject.toml",
      indoc! {
        r#"
        [project]
        name = "Foo_Bar"
        version = "1.0.0"

        [tool.pyproject.rules]
        project-name = "warning"
        "#
      },
    )
    .argument("pyproject.toml")
    .expected_stdout(indoc! {
      r#"
      warning[project-name]: invalid value for `project.name`
         ╭─[ pyproject.toml:2:8 ]
         │
       2 │ name = "Foo_Bar"
         │        ────┬────
         │            ╰────── `project.name` must be PEP 503 normalized (use `foo-bar`)
      ───╯
      "#
    })
    .run()
}
