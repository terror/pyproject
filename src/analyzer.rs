use super::*;

static RULES: &[&dyn Rule] = &[
  &SyntaxRule,
  &SemanticRule,
  &ProjectNameRule,
  &ProjectReadmeRule,
  &ProjectVersionRule,
];

pub(crate) struct Analyzer<'a> {
  document: &'a Document,
}

impl<'a> Analyzer<'a> {
  pub(crate) fn analyze(&self) -> Vec<lsp::Diagnostic> {
    let context = RuleContext::new(self.document);
    RULES.iter().flat_map(|rule| rule.run(&context)).collect()
  }

  pub(crate) fn new(document: &'a Document) -> Self {
    Self { document }
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*, indoc::indoc, pretty_assertions::assert_eq, range::Range,
    std::fs, tempfile::TempDir,
  };

  #[derive(Debug)]
  struct Message<'a> {
    range: (u32, u32, u32, u32),
    text: &'a str,
  }

  #[derive(Debug)]
  struct Test {
    document: Document,
    messages: Vec<(Message<'static>, Option<lsp::DiagnosticSeverity>)>,
    tempdir: Option<TempDir>,
  }

  impl Test {
    fn diagnostic_with_severity(
      self,
      message: Message<'static>,
      severity: Option<lsp::DiagnosticSeverity>,
    ) -> Self {
      Self {
        messages: self
          .messages
          .into_iter()
          .chain([(message, severity)])
          .collect(),
        ..self
      }
    }

    fn error(self, message: Message<'static>) -> Self {
      self
        .diagnostic_with_severity(message, Some(lsp::DiagnosticSeverity::ERROR))
    }

    fn new(content: &str) -> Self {
      Self {
        document: Document::from(content),
        messages: Vec::new(),
        tempdir: None,
      }
    }

    fn run(self) {
      let Test {
        document, messages, ..
      } = self;

      let analyzer = Analyzer::new(&document);

      let diagnostics = analyzer.analyze();

      assert_eq!(
        diagnostics.len(),
        messages.len(),
        "Expected diagnostics {:?} but got {:?}",
        messages,
        diagnostics,
      );

      for (diagnostic, (expected_message, expected_severity)) in
        diagnostics.into_iter().zip(messages.into_iter())
      {
        assert_eq!(diagnostic.message, expected_message.text);
        assert_eq!(diagnostic.range, expected_message.range.range());
        assert_eq!(diagnostic.severity, expected_severity);
      }
    }

    fn with_tempdir(content: &str) -> Self {
      let tempdir = TempDir::new().unwrap();

      let params = lsp::DidOpenTextDocumentParams {
        text_document: lsp::TextDocumentItem {
          language_id: "toml".into(),
          text: content.into(),
          uri: lsp::Url::from_file_path(tempdir.path().join("pyproject.toml"))
            .unwrap(),
          version: 1,
        },
      };

      Self {
        document: Document::from(params),
        messages: Vec::new(),
        tempdir: Some(tempdir),
      }
    }

    fn write_file(self, path: &str, content: &str) -> Self {
      let Some(tempdir) = &self.tempdir else {
        panic!("Test does not have a temporary directory");
      };

      fs::write(tempdir.path().join(path), content).unwrap();

      self
    }
  }

  #[test]
  fn valid_document_has_no_diagnostics() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      "
    })
    .run();
  }

  #[test]
  fn unexpected_entry() {
    Test::new(indoc! {
      "
      FOO
      "
    })
    .error(Message {
      range: (0, 3, 1, 0),
      text: "expected \"=\"",
    })
    .run();
  }

  #[test]
  fn unexpected_value() {
    Test::new(indoc! {
      "
      FOO =
      "
    })
    .error(Message {
      range: (0, 5, 1, 0),
      text: "expected value",
    })
    .run();
  }

  #[test]
  fn conflicting_keys() {
    Test::new(indoc! {
      "
      [foo]
      foo = \"demo\"
      foo = \"example\"

      [bar]
      bar = \"demo\"
      bar = \"example\"
      "
    })
    .error(Message {
      range: (2, 0, 2, 3),
      text: "conflicting keys: `foo` conflicts with `foo`",
    })
    .error(Message {
      range: (6, 0, 6, 3),
      text: "conflicting keys: `bar` conflicts with `bar`",
    })
    .run();
  }

  #[test]
  fn reopening_table_as_array_requires_array_of_tables() {
    Test::new(indoc! {
      "
      [tool]
      name = \"demo\"

      [[tool]]
      name = \"example\"
      "
    })
    .error(Message {
      range: (0, 1, 0, 5),
      text: "expected array of tables `tool` required by `tool`",
    })
    .run();
  }

  #[test]
  fn reopening_scalar_as_table_requires_table() {
    Test::new(indoc! {
      "
      dependencies = \"demo\"

      [dependencies.packages]
      foo = \"bar\"
      "
    })
    .error(Message {
      range: (0, 0, 0, 12),
      text: "expected table `dependencies` required by `dependencies`",
    })
    .run();
  }

  #[test]
  fn redefining_table_header_conflicts() {
    Test::new(indoc! {
      "
      [tool]
      name = \"demo\"

      [tool]
      version = \"1.0.0\"
      "
    })
    .error(Message {
      range: (3, 1, 3, 5),
      text: "conflicting keys: `tool` conflicts with `tool`",
    })
    .run();
  }

  #[test]
  fn invalid_escape_sequences() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\\q\"
      version = \"1.0.0\"
      "
    })
    .error(Message {
      range: (1, 12, 1, 12),
      text: "invalid escape sequence",
    })
    .run();
  }

  #[test]
  fn project_name_must_be_a_string() {
    Test::new(indoc! {
      "
      [project]
      name = 123
      version = \"1.0.0\"
      "
    })
    .error(Message {
      range: (1, 7, 1, 10),
      text: "`project.name` must be a string",
    })
    .run();
  }

  #[test]
  fn project_name_must_not_be_empty() {
    Test::new(indoc! {
      "
      [project]
      name = \"\"
      version = \"1.0.0\"
      "
    })
    .error(Message {
      range: (1, 7, 1, 9),
      text: "`project.name` must not be empty",
    })
    .run();
  }

  #[test]
  fn project_name_must_be_pep_503_normalized() {
    Test::new(indoc! {
      "
      [project]
      name = \"My_Package\"
      version = \"1.0.0\"
      "
    })
    .error(Message {
      range: (1, 7, 1, 19),
      text: "`project.name` must be PEP 503 normalized (use `my-package`)",
    })
    .run();
  }

  #[test]
  fn project_name_is_required() {
    Test::new(indoc! {
      "
      [project]
      version = \"1.0.0\"
      "
    })
    .error(Message {
      range: (0, 0, 0, 9),
      text: "missing required key `project.name`",
    })
    .run();
  }

  #[test]
  fn project_version_must_be_a_string() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = 1
      "
    })
    .error(Message {
      range: (2, 10, 2, 11),
      text: "`project.version` must be a string",
    })
    .run();
  }

  #[test]
  fn project_version_must_not_be_empty() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"\"
      "
    })
    .error(Message {
      range: (2, 10, 2, 12),
      text: "`project.version` must not be empty",
    })
    .run();
  }

  #[test]
  fn project_version_must_be_pep_440_compliant() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"foo\"
      "
    })
    .error(Message {
      range: (2, 10, 2, 15),
      text: "expected version to start with a number, but no leading ASCII digits were found",
    })
    .run();
  }

  #[test]
  fn project_version_is_required_unless_dynamic() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      "
    })
    .error(Message {
      range: (0, 0, 0, 9),
      text: "missing required key `project.version`",
    })
    .run();

    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      dynamic = [\"version\"]
      "
    })
    .run();
  }

  #[test]
  fn project_readme_string_must_point_to_existing_file() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      readme = \"README.md\"
      "
    })
    .error(Message {
      range: (3, 9, 3, 20),
      text: "file `README.md` for `project.readme` does not exist",
    })
    .run();
  }

  #[test]
  fn project_readme_string_requires_known_extension() {
    Test::with_tempdir(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      readme = \"README.txt\"
      "
    })
    .write_file("README.txt", "# readme")
    .error(Message {
      range: (3, 9, 3, 21),
      text: "`project.readme` must point to a `.md` or `.rst` file when specified as a string",
    })
    .run();
  }

  #[test]
  fn project_readme_string_path_must_be_relative() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      readme = \"/README.md\"
      "
    })
    .error(Message {
      range: (3, 9, 3, 21),
      text: "file path for `project.readme` must be relative",
    })
    .error(Message {
      range: (3, 9, 3, 21),
      text: "file `/README.md` for `project.readme` does not exist",
    })
    .run();
  }

  #[test]
  fn project_readme_table_requires_content_type() {
    Test::with_tempdir(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      readme = { file = \"README.md\" }
      "
    })
    .write_file("README.md", "# readme")
    .error(Message {
      range: (3, 9, 3, 31),
      text: "missing required key `project.readme.content-type`",
    })
    .run();
  }

  #[test]
  fn project_readme_table_requires_file_or_text() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      readme = { content-type = \"text/markdown\" }
      "
    })
    .error(Message {
      range: (3, 9, 3, 43),
      text: "missing required key `project.readme.file` or `project.readme.text`",
    })
    .run();
  }

  #[test]
  fn project_readme_table_must_not_mix_file_and_text() {
    Test::with_tempdir(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      readme = { file = \"README.md\", text = \"inline\", content-type = \"text/markdown\" }
      "
    })
    .write_file("README.md", "# readme")
    .error(Message {
      range: (3, 9, 3, 80),
      text: "`project.readme` must specify only one of `file` or `text`",
    })
    .run();
  }

  #[test]
  fn project_readme_table_text_must_be_a_string() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      readme = { text = 1, content-type = \"text/markdown\" }
      "
    })
    .error(Message {
      range: (3, 18, 3, 19),
      text: "`project.readme.text` must be a string",
    })
    .run();
  }

  #[test]
  fn project_readme_table_file_must_exist() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      readme = { file = \"README.md\", content-type = \"text/markdown\" }
      "
    })
    .error(Message {
      range: (3, 18, 3, 29),
      text: "file `README.md` for `project.readme` does not exist",
    })
    .run();
  }

  #[test]
  fn valid_project_readme_table_with_content_type_and_file() {
    Test::with_tempdir(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      readme = { file = \"README.md\", content-type = \"text/markdown\" }
      "
    })
    .write_file("README.md", "# readme")
    .run();
  }
}
