use super::*;

static RULES: &[&dyn Rule] = &[
  &SyntaxRule,
  &SemanticRule,
  &ProjectNameRule,
  &ProjectDescriptionRule,
  &ProjectLicenseRule,
  &ProjectClassifiersRule,
  &ProjectKeywordsRule,
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

    fn warning(self, message: Message<'static>) -> Self {
      self.diagnostic_with_severity(
        message,
        Some(lsp::DiagnosticSeverity::WARNING),
      )
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
  fn project_description_must_be_a_string() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      description = [\"not a string\"]
      "
    })
    .error(Message {
      range: (3, 14, 3, 30),
      text: "`project.description` must be a string",
    })
    .run();
  }

  #[test]
  fn project_keywords_must_be_an_array_of_strings() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      keywords = \"invalid\"
      "
    })
    .error(Message {
      range: (3, 11, 3, 20),
      text: "`project.keywords` must be an array of strings",
    })
    .run();
  }

  #[test]
  fn project_keywords_items_must_be_strings() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      keywords = [1, \"two\"]
      "
    })
    .error(Message {
      range: (3, 12, 3, 13),
      text: "`project.keywords` items must be strings",
    })
    .run();
  }

  #[test]
  fn project_classifiers_must_be_an_array_of_strings() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      classifiers = \"invalid\"
      "
    })
    .error(Message {
      range: (3, 14, 3, 23),
      text: "`project.classifiers` must be an array of strings",
    })
    .run();
  }

  #[test]
  fn project_classifiers_items_must_be_strings() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      classifiers = [1]
      "
    })
    .error(Message {
      range: (3, 15, 3, 16),
      text: "`project.classifiers` items must be strings",
    })
    .run();
  }

  #[test]
  fn project_classifiers_must_use_known_values() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      classifiers = [\"Not Real :: Classifier\"]
      "
    })
    .error(Message {
      range: (3, 15, 3, 39),
      text: "`project.classifiers` contains an unknown classifier `Not Real :: Classifier`",
    })
    .run();
  }

  #[test]
  fn project_classifiers_accept_known_values() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      classifiers = [
        \"Development Status :: 4 - Beta\",
        \"Intended Audience :: Developers\",
        \"Programming Language :: Python :: 3\",
        \"Programming Language :: Python :: 3.12\",
      ]
      "
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
  fn project_license_table_requires_file_or_text() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = { }
      "
    })
    .warning(Message {
      range: (3, 10, 3, 13),
      text: "`project.license` tables are deprecated; prefer a SPDX expression string and `project.license-files`",
    })
    .error(Message {
      range: (3, 10, 3, 13),
      text: "missing required key `project.license.file` or `project.license.text`",
    })
    .run();
  }

  #[test]
  fn project_license_table_must_not_mix_file_and_text() {
    Test::with_tempdir(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = { file = \"LICENSE\", text = \"Apache\" }
      "
    })
    .write_file("LICENSE", "MIT")
    .warning(Message {
      range: (3, 10, 3, 47),
      text: "`project.license` tables are deprecated; prefer a SPDX expression string and `project.license-files`",
    })
    .error(Message {
      range: (3, 10, 3, 47),
      text: "`project.license` must specify only one of `file` or `text`",
    })
    .run();
  }

  #[test]
  fn project_license_table_file_must_be_a_string() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = { file = 1 }
      "
    })
    .warning(Message {
      range: (3, 10, 3, 22),
      text: "`project.license` tables are deprecated; prefer a SPDX expression string and `project.license-files`",
    })
    .error(Message {
      range: (3, 19, 3, 20),
      text: "`project.license.file` must be a string",
    })
    .run();
  }

  #[test]
  fn project_license_table_text_must_be_a_string() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = { text = 1 }
      "
    })
    .warning(Message {
      range: (3, 10, 3, 22),
      text: "`project.license` tables are deprecated; prefer a SPDX expression string and `project.license-files`",
    })
    .error(Message {
      range: (3, 19, 3, 20),
      text: "`project.license.text` must be a string",
    })
    .run();
  }

  #[test]
  fn project_license_table_file_path_must_be_relative() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = { file = \"/LICENSE\" }
      "
    })
    .warning(Message {
      range: (3, 10, 3, 31),
      text: "`project.license` tables are deprecated; prefer a SPDX expression string and `project.license-files`",
    })
    .error(Message {
      range: (3, 19, 3, 29),
      text: "file path for `project.license.file` must be relative",
    })
    .error(Message {
      range: (3, 19, 3, 29),
      text: "file `/LICENSE` for `project.license.file` does not exist",
    })
    .run();
  }

  #[test]
  fn project_license_table_file_must_exist() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = { file = \"LICENSE\" }
      "
    })
    .warning(Message {
      range: (3, 10, 3, 30),
      text: "`project.license` tables are deprecated; prefer a SPDX expression string and `project.license-files`",
    })
    .error(Message {
      range: (3, 19, 3, 28),
      text: "file `LICENSE` for `project.license.file` does not exist",
    })
    .run();
  }

  #[test]
  fn valid_project_license_table_with_file() {
    Test::with_tempdir(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = { file = \"LICENSE\" }
      "
    })
    .write_file("LICENSE", "MIT")
    .warning(Message {
      range: (3, 10, 3, 30),
      text: "`project.license` tables are deprecated; prefer a SPDX expression string and `project.license-files`",
    })
    .run();
  }

  #[test]
  fn project_license_string_must_not_be_empty() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"\"
      "
    })
    .error(Message {
      range: (3, 10, 3, 12),
      text: "`project.license` must not be empty",
    })
    .run();
  }

  #[test]
  fn project_license_must_be_string_or_table() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = []
      "
    })
    .error(Message {
      range: (3, 10, 3, 12),
      text: "`project.license` must be a string or table",
    })
    .run();
  }

  #[test]
  fn project_license_must_be_valid_spdx_expression() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"Apache-2.0 OR NotARealLicense\"
      "
    })
    .error(Message {
      range: (3, 10, 3, 41),
      text: "`project.license` must be a valid SPDX expression: unknown term",
    })
    .run();
  }

  #[test]
  fn project_license_suggests_canonical_expression() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"mit\"
      "
    })
    .error(Message {
      range: (3, 10, 3, 15),
      text: "`project.license` must be a valid SPDX expression: unknown term (did you mean `MIT`?)",
    })
    .run();
  }

  #[test]
  fn project_license_warns_on_deprecated_identifier() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"wxWindows\"
      "
    })
    .warning(Message {
      range: (3, 10, 3, 21),
      text: "license identifier `wxWindows` in `project.license` is deprecated",
    })
    .run();
  }

  #[test]
  fn project_license_classifiers_forbidden_when_license_set() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"MIT\"
      classifiers = [
        \"License :: OSI Approved :: MIT License\",
        \"Programming Language :: Python\",
      ]
      "
    })
    .warning(Message {
      range: (5, 2, 5, 42),
      text: "`project.classifiers` license classifiers are deprecated when `project.license` is present (use only `project.license`)",
    })
    .error(Message {
      range: (4, 14, 7, 1),
      text: "`project.classifiers` must not include license classifiers when `project.license` is set",
    })
    .run();
  }

  #[test]
  fn project_license_classifiers_warn_without_license() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      classifiers = [\"License :: OSI Approved :: MIT License\"]
      "
    })
    .warning(Message {
      range: (3, 15, 3, 55),
      text: "`project.classifiers` license classifiers are deprecated; use `project.license` instead",
    })
    .run();
  }

  #[test]
  fn project_license_files_must_be_array_of_strings() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"MIT\"
      license-files = \"LICENSE*\"
      "
    })
    .error(Message {
      range: (4, 16, 4, 26),
      text: "`project.license-files` must be an array of strings",
    })
    .run();
  }

  #[test]
  fn project_license_files_items_must_be_strings() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"MIT\"
      license-files = [1]
      "
    })
    .error(Message {
      range: (4, 17, 4, 18),
      text: "`project.license-files` items must be strings",
    })
    .run();
  }

  #[test]
  fn project_license_files_rejects_invalid_patterns() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"MIT\"
      license-files = [\"/LICENSE\"]
      "
    })
    .error(Message {
      range: (4, 17, 4, 27),
      text: "invalid `project.license-files` pattern `/LICENSE`: patterns must be relative; leading `/` is not allowed",
    })
    .run();
  }

  #[test]
  fn project_license_files_rejects_parent_segments() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"MIT\"
      license-files = [\"..\\\\LICENSE\"]
      "
    })
    .error(Message {
      range: (4, 17, 4, 30),
      text: "invalid `project.license-files` pattern `..\\LICENSE`: path delimiter must be `/`, not `\\`",
    })
    .run();
  }

  #[test]
  fn project_license_files_pattern_must_match() {
    Test::with_tempdir(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"MIT\"
      license-files = [\"LICENSE*\"]"
    })
    .error(Message {
      range: (4, 17, 4, 27),
      text: "`project.license-files` pattern `LICENSE*` did not match any files",
    })
    .run();
  }

  #[test]
  fn project_license_files_pattern_allows_empty_array() {
    Test::new(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"MIT\"
      license-files = []
      "
    })
    .run();
  }

  #[test]
  fn project_license_files_must_point_to_existing_utf8_files() {
    Test::with_tempdir(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"MIT\"
      license-files = [\"LICENSE\"]
      "
    })
    .error(Message {
      range: (4, 17, 4, 26),
      text: "`project.license-files` pattern `LICENSE` did not match any files",
    })
    .run();
  }

  #[test]
  fn project_license_files_accepts_valid_match() {
    Test::with_tempdir(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = \"MIT\"
      license-files = [\"LICENSE\"]
      "
    })
    .write_file("LICENSE", "MIT")
    .run();
  }

  #[test]
  fn project_license_files_requires_string_license_when_present() {
    Test::with_tempdir(indoc! {
      "
      [project]
      name = \"demo\"
      version = \"1.0.0\"
      license = { file = \"LICENSE\" }
      license-files = [\"LICENSE\"]
      "
    })
    .write_file("LICENSE", "MIT")
    .error(Message {
      range: (3, 10, 3, 30),
      text: "`project.license` must be a string SPDX expression when `project.license-files` is present",
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
