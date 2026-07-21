use {
  anyhow::Error,
  executable_path::executable_path,
  indoc::{formatdoc, indoc},
  pretty_assertions::assert_eq,
  std::{fs, iter::once, path::PathBuf, process::Command, str},
  tempfile::TempDir,
};

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
struct Test<'a> {
  arguments: Vec<String>,
  directory: Option<String>,
  expected_files: Vec<(&'a str, &'a str)>,
  expected_status: i32,
  expected_stderr: String,
  expected_stdout: String,
  files: Vec<(&'a str, &'a str)>,
  subcommand: String,
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
      .arg(&self.subcommand)
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

  fn expected_file(self, path: &'a str, content: &'a str) -> Self {
    Self {
      expected_files: self
        .expected_files
        .into_iter()
        .chain(once((path, content)))
        .collect(),
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
      expected_files: Vec::new(),
      expected_status: 0,
      expected_stderr: String::new(),
      expected_stdout: String::new(),
      files: Vec::new(),
      subcommand: "check".to_owned(),
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

    for (path, expected) in self.expected_files {
      let actual = fs::read_to_string(self.tempdir.path().join(path))?;

      assert_eq!(actual, expected, "unexpected content for `{path}`");
    }

    Ok(())
  }

  fn subcommand(self, subcommand: &str) -> Self {
    Self {
      subcommand: subcommand.to_owned(),
      ..self
    }
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
fn check_configured_rule_severities() -> Result {
  #[track_caller]
  fn case(level: &str) -> Result {
    let content = formatdoc! {
      r#"
      [project]
      name = "Foo!Bar"
      version = "1.0.0"

      [tool.pyproject.rules]
      project-name = "{level}"
      "#
    };

    let expected_stdout = if level == "off" {
      String::new()
    } else {
      formatdoc! {
        r#"
        {level}[project-name]: invalid value for `project.name`
           ╭─[ pyproject.toml:2:8 ]
           │
         2 │ name = "Foo!Bar"
           │        ────┬────
           │            ╰────── `project.name` must be a valid distribution name
        ───╯
        "#
      }
    };

    Test::new()?
      .file("pyproject.toml", &content)
      .argument("pyproject.toml")
      .expected_stdout(&expected_stdout)
      .run()
  }

  case("off")?;
  case("hint")?;
  case("info")?;
  case("warning")
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
fn check_multiple_diagnostics_are_sorted_and_fail() -> Result {
  Test::new()?
    .file(
      "pyproject.toml",
      indoc! {
        r#"
        [project]
        name = "Foo!Bar"
        version = "foo"

        [tool.pyproject.rules]
        project-name = "warning"
        "#
      },
    )
    .argument("pyproject.toml")
    .expected_status(1)
    .expected_stdout(indoc! {
      r#"
      warning[project-name]: invalid value for `project.name`
         ╭─[ pyproject.toml:2:8 ]
         │
       2 │ name = "Foo!Bar"
         │        ────┬────
         │            ╰────── `project.name` must be a valid distribution name
      ───╯
      error[project-version]: invalid `project.version` value
         ╭─[ pyproject.toml:3:11 ]
         │
       3 │ version = "foo"
         │           ──┬──
         │             ╰──── expected version to start with a number, but no leading ASCII digits were found
      ───╯
      "#
    })
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
        name = "Foo!Bar"
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
       2 │ name = "Foo!Bar"
         │        ────┬────
         │            ╰────── `project.name` must be a valid distribution name
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
        name = "Foo!Bar"
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
       2 │ name = "Foo!Bar"
         │        ────┬────
         │            ╰────── `project.name` must be a valid distribution name
      ───╯
      "#
    })
    .run()
}

#[test]
fn check_uses_command_line_schema() -> Result {
  let test = Test::new()?;

  let schema = format!(
    "foo={}",
    tower_lsp::lsp_types::Url::from_file_path(
      test.tempdir.path().join("foo.json")
    )
    .unwrap()
  );

  test
    .file(
      "foo.json",
      indoc! {
        r#"
        {
          "$id": "file:///foo.json",
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "enabled": { "type": "boolean" }
          }
        }
        "#
      },
    )
    .file(
      "pyproject.toml",
      indoc! {
        r#"
        [tool.foo]
        enabled = "bar"
        "#
      },
    )
    .argument("--schema")
    .argument(&schema)
    .argument("pyproject.toml")
    .expected_status(1)
    .expected_stdout(indoc! {
      r#"
      error[json-schema]: schema mismatch
         ╭─[ pyproject.toml:2:1 ]
         │
       2 │ enabled = "bar"
         │ ───────┬───────
         │        ╰───────── expected boolean for `tool.foo.enabled`, got string "bar"
      ───╯
      "#
    })
    .run()
}

#[test]
fn format_check_errors_for_unformatted_file() -> Result {
  Test::new()?
    .subcommand("format")
    .file(
      "pyproject.toml",
      indoc! {
        r#"
        [project]
        name="foo"
        version="1.0.0"
        "#
      },
    )
    .argument("--check")
    .expected_status(1)
    .expected_stdout(concat!(
      "--- [ROOT]/pyproject.toml\n",
      "+++ [ROOT]/pyproject.toml (formatted)\n",
      "@@ -1,3 +1,3 @@\n",
      " [project]\n",
      "-name=\"foo\"\n",
      "-version=\"1.0.0\"\n",
      "+name = \"foo\"\n",
      "+version = \"1.0.0\"\n",
    ))
    .run()
}

#[test]
fn format_prints_formatted_file() -> Result {
  Test::new()?
    .subcommand("format")
    .file(
      "pyproject.toml",
      indoc! {
        r#"
        [project]
        name="foo"
        version="1.0.0"
        "#
      },
    )
    .expected_stdout(indoc! {
      r#"
      [project]
      name = "foo"
      version = "1.0.0"
      "#
    })
    .run()
}

#[test]
fn format_write_formats_file() -> Result {
  Test::new()?
    .subcommand("format")
    .file(
      "pyproject.toml",
      indoc! {
        r#"
        [project]
        name="foo"
        version="1.0.0"
        "#
      },
    )
    .argument("--write")
    .expected_file(
      "pyproject.toml",
      indoc! {
        r#"
        [project]
        name = "foo"
        version = "1.0.0"
        "#
      },
    )
    .run()
}
