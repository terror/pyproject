use super::*;

static RULES: &[&dyn Rule] = &[
  &SyntaxRule,
  &SemanticRule,
  &SchemaRule,
  &ProjectUnknownKeysRule,
  &DependencyGroupsRule,
  &ProjectDynamicRule,
  &ProjectDependencyDeprecationsRule,
  &ProjectDependenciesRule,
  &ProjectDependenciesVersionBoundsRule,
  &ProjectDependencyUpdatesRule,
  &ProjectOptionalDependenciesRule,
  &ProjectImportNamesRule,
  &ProjectNameRule,
  &ProjectDescriptionRule,
  &ProjectEntryPointsRule,
  &ProjectEntryPointsImportableRule,
  &ProjectEntryPointsExtrasRule,
  &ProjectLicenseValueDeprecationsRule,
  &ProjectLicenseValueRule,
  &ProjectLicenseFilesRule,
  &ProjectLicenseClassifiersDeprecatedRule,
  &ProjectLicenseClassifiersRule,
  &ProjectClassifiersRule,
  &ProjectKeywordsRule,
  &ProjectPeopleRule,
  &ProjectUrlsRule,
  &ProjectReadmeRule,
  &ProjectReadmeContentTypeRule,
  &ProjectRequiresPythonRule,
  &ProjectRequiresPythonUpperBoundRule,
  &ProjectVersionRule,
];

pub(crate) struct Analyzer<'a> {
  document: &'a Document,
}

impl<'a> Analyzer<'a> {
  pub(crate) fn analyze(&self) -> Vec<Diagnostic> {
    let context = RuleContext::new(self.document);

    let config = &self.document.config;

    RULES
      .par_iter()
      .flat_map(|rule| {
        let rule_config = config.rule_config(rule.id());

        rule
          .run(&context)
          .into_iter()
          .filter_map(move |diagnostic| {
            rule_config
              .severity(diagnostic.severity, rule.default_level())
              .map(|severity| Diagnostic {
                display: rule.display().to_string(),
                id: rule.id().to_string(),
                severity,
                ..diagnostic
              })
          })
          .collect::<Vec<Diagnostic>>()
      })
      .collect()
  }

  pub(crate) fn new(document: &'a Document) -> Self {
    Self { document }
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*, crate::pypi_client::set_mock_latest_version, indoc::indoc,
    pretty_assertions::assert_eq, std::fs, tempfile::TempDir,
  };

  #[derive(Debug)]
  struct Message<'a> {
    range: (u32, u32, u32, u32),
    text: &'a str,
  }

  #[derive(Debug)]
  struct Test {
    document: Document,
    messages: Vec<(Message<'static>, lsp::DiagnosticSeverity)>,
    tempdir: Option<TempDir>,
  }

  impl Test {
    fn diagnostic(
      self,
      message: Message<'static>,
      severity: lsp::DiagnosticSeverity,
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
      self.diagnostic(message, lsp::DiagnosticSeverity::ERROR)
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

    fn set_package_latest_version(self, name: &str, version: &str) -> Self {
      set_mock_latest_version(name, Some(version));
      self
    }

    fn warning(self, message: Message<'static>) -> Self {
      self.diagnostic(message, lsp::DiagnosticSeverity::WARNING)
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

      let path = tempdir.path().join(path);

      if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
      }

      fs::write(path, content).unwrap();

      self
    }
  }

  #[test]
  fn valid_document_has_no_diagnostics() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      "#
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
      r#"
      [foo]
      foo = "demo"
      foo = "example"

      [bar]
      bar = "demo"
      bar = "example"
      "#
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
  fn project_readme_rejects_unknown_keys() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = { text = "hi", content-type = "text/markdown", extra = "nope" }
      "#
    })
    .error(Message {
      range: (3, 56, 3, 61),
      text: "`project.readme` only supports `file`, `text`, and `content-type` keys",
    })
    .run();
  }

  #[test]
  fn reopening_table_as_array_requires_array_of_tables() {
    Test::new(indoc! {
      r#"
      [tool]
      name = "demo"

      [[tool]]
      name = "example"
      "#
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
      r#"
      dependencies = "demo"

      [dependencies.packages]
      foo = "bar"
      "#
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
      r#"
      [tool]
      name = "demo"

      [tool]
      version = "1.0.0"
      "#
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
      r#"
      [project]
      name = "demo"
      description = "demo\q"
      version = "1.0.0"
      "#
    })
    .error(Message {
      range: (2, 19, 2, 19),
      text: "invalid escape sequence",
    })
    .run();
  }

  #[test]
  fn project_name_must_be_a_string() {
    Test::new(indoc! {
      r#"
      [project]
      name = 123
      version = "1.0.0"
      "#
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
      r#"
      [project]
      name = ""
      version = "1.0.0"
      "#
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
      r#"
      [project]
      name = "My_Package"
      version = "1.0.0"
      "#
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
      r#"
      [project]
      version = "1.0.0"
      "#
    })
    .error(Message {
      range: (0, 0, 0, 9),
      text: "missing required key `project.name`",
    })
    .run();
  }

  #[test]
  fn rule_can_be_disabled_in_configuration() {
    Test::new(indoc! {
      r#"
      [project]
      version = "1.0.0"

      [tool.pyproject.rules]
      project-name = "off"
      "#
    })
    .run();
  }

  #[test]
  fn rule_severity_can_be_overridden() {
    Test::new(indoc! {
      r#"
      [project]
      name = "My_Package"
      version = "1.0.0"

      [tool.pyproject.rules]
      project-name = "warning"
      "#
    })
    .warning(Message {
      range: (1, 7, 1, 19),
      text: "`project.name` must be PEP 503 normalized (use `my-package`)",
    })
    .run();
  }

  #[test]
  fn rule_severity_can_be_overridden_with_table() {
    Test::new(indoc! {
      r#"
      [project]
      name = "My_Package"
      version = "1.0.0"

      [tool.pyproject.rules.project-name]
      level = "warning"
      "#
    })
    .warning(Message {
      range: (1, 7, 1, 19),
      text: "`project.name` must be PEP 503 normalized (use `my-package`)",
    })
    .run();
  }

  #[test]
  fn project_description_must_be_a_string() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      description = ["not a string"]
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      keywords = "invalid"
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      keywords = [1, "two"]
      "#
    })
    .error(Message {
      range: (3, 12, 3, 13),
      text: "`project.keywords` items must be strings",
    })
    .run();
  }

  #[test]
  fn project_keywords_must_not_contain_duplicates() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      keywords = ["one", "two", "one"]
      "#
    })
    .error(Message {
      range: (3, 26, 3, 31),
      text: "`project.keywords` contains duplicate keyword `one`",
    })
    .run();
  }

  #[test]
  fn project_classifiers_must_not_contain_duplicates() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      classifiers = [
        "Environment :: Web Environment",
        "Framework :: Pylons",
        "Framework :: Pylons",
      ]
      "#
    })
    .error(Message {
      range: (6, 2, 6, 23),
      text: "`project.classifiers` contains duplicate classifier `Framework :: Pylons`",
    })
    .run();
  }

  #[test]
  fn project_dependencies_must_be_array_of_strings() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dependencies = "requests"
      "#
    })
    .error(Message {
      range: (3, 15, 3, 25),
      text: "`project.dependencies` must be an array of PEP 508 strings",
    })
    .run();
  }

  #[test]
  fn project_dependencies_items_must_be_strings() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dependencies = [1]
      "#
    })
    .error(Message {
      range: (3, 16, 3, 17),
      text: "`project.dependencies` items must be strings",
    })
    .run();
  }

  #[test]
  fn project_dependencies_rejects_invalid_specifier() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dependencies = ["requests >= "]
      "#
    })
    .error(Message {
      range: (3, 16, 3, 30),
      text: "`project.dependencies` item `requests >= ` is not a valid PEP 508 dependency: unexpected end of version specifier, expected version",
    })
    .run();
  }

  #[test]
  fn project_dependencies_require_normalized_names() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dependencies = ["Requests>=1.0"]
      "#
    })
    .error(Message {
      range: (3, 16, 3, 31),
      text: "`project.dependencies` package name `Requests` must be normalized (use `requests`)",
    })
    .run();
  }

  #[test]
  fn project_dependencies_warn_on_insecure_and_unbounded() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dependencies = ["pycrypto"]
      "#
    })
    .warning(Message {
      range: (3, 16, 3, 26),
      text: "`project.dependencies` includes deprecated/insecure package `pycrypto`: package is unmaintained and insecure; consider `pycryptodome`",
    })
    .run();
  }

  #[test]
  fn project_dependencies_warn_on_insecure_and_unbounded_when_enabled() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dependencies = ["pycrypto"]

      [tool.pyproject.rules]
      project-dependencies-version-bounds = "warning"
      "#
    })
    .warning(Message {
      range: (3, 16, 3, 26),
      text: "`project.dependencies` includes deprecated/insecure package `pycrypto`: package is unmaintained and insecure; consider `pycryptodome`",
    })
    .warning(Message {
      range: (3, 16, 3, 26),
      text: "`project.dependencies` entry `pycrypto` does not pin a version; add a version range with an upper bound to avoid future breaking changes",
    })
    .run();
  }

  #[test]
  fn project_dependencies_version_bounds_opt_in() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dependencies = ["requests>=1.0"]
      "#
    })
    .run();
  }

  #[test]
  fn project_dependencies_warn_without_upper_bound_when_enabled() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dependencies = ["requests>=1.0"]

      [tool.pyproject.rules]
      project-dependencies-version-bounds = "warning"
      "#
    })
    .warning(Message {
      range: (3, 16, 3, 31),
      text: "`project.dependencies` entry `requests` does not specify an upper version bound; consider adding an upper constraint to avoid future breaking changes",
    })
    .run();
  }

  #[test]
  fn project_dependencies_warn_when_latest_release_is_excluded() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dependencies = ["requests>=1,<2"]
      "#
    })
    .set_package_latest_version("requests", "3.0.0")
    .warning(Message {
      range: (3, 16, 3, 32),
      text: "`project.dependencies` entry `requests` excludes the latest release `3.0.0` (current constraint: `>=1, <2`)",
    })
    .run();
  }

  #[test]
  fn project_optional_dependencies_must_be_table() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      [project.optional-dependencies]
      test = "not an array"
      "#
    })
    .error(Message {
      range: (4, 7, 4, 21),
      text: "`project.optional-dependencies.test` must be an array of PEP 508 strings",
    })
    .run();
  }

  #[test]
  fn project_optional_dependencies_must_be_table_when_string() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      optional-dependencies = "not a table"
      "#
    })
    .error(Message {
      range: (3, 24, 3, 37),
      text: "`project.optional-dependencies` must be a table",
    })
    .run();
  }

  #[test]
  fn project_optional_dependencies_items_must_be_strings() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      [project.optional-dependencies]
      test = [1]
      "#
    })
    .error(Message {
      range: (4, 8, 4, 9),
      text: "`project.optional-dependencies.test[0]` must be a string",
    })
    .run();
  }

  #[test]
  fn project_optional_dependencies_rejects_invalid_specifier() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      [project.optional-dependencies]
      test = ["requests >= "]
      "#
    })
    .error(Message {
      range: (4, 8, 4, 22),
      text: "`project.optional-dependencies.test[0]` item `requests >= ` is not a valid PEP 508 dependency: unexpected end of version specifier, expected version",
    })
    .run();
  }

  #[test]
  fn project_optional_dependencies_require_normalized_names() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      [project.optional-dependencies]
      test = ["Requests>=1.0"]
      "#
    })
    .error(Message {
      range: (4, 8, 4, 23),
      text: "`project.optional-dependencies.test[0]` package name `Requests` must be normalized (use `requests`)",
    })
    .run();
  }

  #[test]
  fn project_optional_dependencies_rejects_invalid_extra_name() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      [project.optional-dependencies]
      "invalid-extra-name!" = ["requests"]
      "#
    })
    .error(Message {
      range: (4, 0, 4, 21),
      text: "`project.optional-dependencies.invalid-extra-name!` key `invalid-extra-name!` must be a valid PEP 508 extra name",
    })
    .run();
  }

  #[test]
  fn project_optional_dependencies_valid_configuration() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      [project.optional-dependencies]
      test = ["pytest>=7.0.0", "pytest-cov"]
      dev = ["black", "mypy>=1.0.0"]
      "#
    })
    .run();
  }

  #[test]
  fn project_optional_dependencies_empty_array_valid() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      [project.optional-dependencies]
      test = []
      "#
    })
    .run();
  }

  #[test]
  fn project_optional_dependencies_multiple_errors() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      [project.optional-dependencies]
      "invalid!" = ["Requests>=1.0"]
      test = ["invalid spec >= "]
      "#
    })
    .error(Message {
      range: (4, 0, 4, 10),
      text: "`project.optional-dependencies.invalid!` key `invalid!` must be a valid PEP 508 extra name",
    })
    .error(Message {
      range: (5, 8, 5, 26),
      text: "`project.optional-dependencies.test[0]` item `invalid spec >= ` is not a valid PEP 508 dependency: expected one of `@`, `(`, `<`, `=`, `>`, `~`, `!`, `;`, found `s`",
    })
    .run();
  }

  #[test]
  fn project_authors_must_be_array_of_inline_tables() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      authors = "not an array"
      "#
    })
    .error(Message {
      range: (3, 10, 3, 24),
      text: "`project.authors` must be an array of inline tables",
    })
    .run();
  }

  #[test]
  fn project_authors_items_must_be_inline_tables() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      authors = ["Someone"]
      "#
    })
    .error(Message {
      range: (3, 11, 3, 20),
      text: "`project.authors` items must be inline tables",
    })
    .run();
  }

  #[test]
  fn project_authors_items_only_allow_name_and_email() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      authors = [{foo = "bar"}]
      "#
    })
    .error(Message {
      range: (3, 12, 3, 15),
      text: "`project.authors` items may only contain `name` or `email`",
    })
    .run();
  }

  #[test]
  fn project_authors_name_must_be_string() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      authors = [{name = 123}]
      "#
    })
    .error(Message {
      range: (3, 19, 3, 22),
      text: "`project.authors.name` must be a string",
    })
    .run();
  }

  #[test]
  fn project_authors_name_must_be_valid_email_name() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      authors = [{name = "Last, First"}]
      "#
    })
    .error(Message {
      range: (3, 19, 3, 32),
      text: "`project.authors.name` must be a valid email name without commas",
    })
    .run();
  }

  #[test]
  fn project_authors_email_must_be_valid_address() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      authors = [{email = "not-an-email"}]
      "#
    })
    .error(Message {
      range: (3, 20, 3, 34),
      text: "`project.authors.email` must be a valid email address",
    })
    .run();
  }

  #[test]
  fn project_maintainers_must_be_array_of_inline_tables() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      maintainers = 123
      "#
    })
    .error(Message {
      range: (3, 14, 3, 17),
      text: "`project.maintainers` must be an array of inline tables",
    })
    .run();
  }

  #[test]
  fn project_classifiers_must_be_an_array_of_strings() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      classifiers = "invalid"
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      classifiers = [1]
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      classifiers = ["Not Real :: Classifier"]
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      classifiers = [
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.12",
      ]
      "#
    })
    .run();
  }

  #[test]
  fn project_version_must_be_a_string() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = 1
      "#
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
      r#"
      [project]
      name = "demo"
      version = ""
      "#
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
      r#"
      [project]
      name = "demo"
      version = "foo"
      "#
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
      r#"
      [project]
      name = "demo"
      "#
    })
    .error(Message {
      range: (0, 0, 0, 9),
      text: "missing required key `project.version`",
    })
    .run();

    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      dynamic = ["version"]
      "#
    })
    .run();
  }

  #[test]
  fn project_requires_python_must_be_a_string() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      requires-python = 3.11
      "#
    })
    .error(Message {
      range: (3, 18, 3, 22),
      text: "`project.requires-python` must be a string",
    })
    .run();
  }

  #[test]
  fn project_requires_python_must_not_be_empty() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      requires-python = ""
      "#
    })
    .error(Message {
      range: (3, 18, 3, 20),
      text: "`project.requires-python` must not be empty",
    })
    .run();
  }

  #[test]
  fn project_requires_python_must_be_valid_pep_440() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      requires-python = "=>3.12"
      "#
    })
    .error(Message {
      range: (3, 18, 3, 26),
      text: "`project.requires-python` must be a valid PEP 440 version specifier: Failed to parse version: no such comparison operator \"=>\", must be one of ~= == != <= >= < > ===:\n=>3.12\n^^^^^^\n",
    })
    .run();
  }

  #[test]
  fn project_requires_python_upper_bound_is_opt_in() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      requires-python = ">=3.8"
      "#
    })
    .run();
  }

  #[test]
  fn project_requires_python_warns_without_upper_bound_when_enabled() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      requires-python = ">=3.8"

      [tool.pyproject.rules]
      project-requires-python-bounds = "warning"
      "#
    })
    .warning(Message {
      range: (3, 18, 3, 25),
      text: "`project.requires-python` does not specify an upper bound; consider adding one to avoid unsupported future Python versions",
    })
    .run();
  }

  #[test]
  fn project_requires_python_allows_upper_bound_or_exact() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      requires-python = ">=3.10, <4"
      "#
    })
    .run();
  }

  #[test]
  fn project_requires_python_respects_dynamic() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dynamic = ["requires-python"]
      "#
    })
    .error(Message {
      range: (3, 11, 3, 28),
      text: "`project.dynamic` contains unsupported field `requires-python`",
    })
    .run();
  }

  #[test]
  fn project_dynamic_must_be_array_of_strings() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dynamic = "version"
      "#
    })
    .error(Message {
      range: (3, 10, 3, 19),
      text: "`project.dynamic` must be an array of strings",
    })
    .run();
  }

  #[test]
  fn project_dynamic_items_must_be_strings() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dynamic = [1]
      "#
    })
    .error(Message {
      range: (3, 11, 3, 12),
      text: "`project.dynamic` items must be strings",
    })
    .run();
  }

  #[test]
  fn project_dynamic_disallows_requires_python() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dynamic = ["requires-python"]
      "#
    })
    .error(Message {
      range: (3, 11, 3, 28),
      text: "`project.dynamic` contains unsupported field `requires-python`",
    })
    .run();
  }

  #[test]
  fn project_dynamic_rejects_unsupported_fields() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      dynamic = ["version", "foo"]
      "#
    })
    .error(Message {
      range: (2, 22, 2, 27),
      text: "`project.dynamic` contains unsupported field `foo`",
    })
    .run();
  }

  #[test]
  fn project_dynamic_must_not_include_name() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dynamic = ["name"]
      "#
    })
    .error(Message {
      range: (3, 11, 3, 17),
      text: "`project.dynamic` must not include `name`",
    })
    .run();
  }

  #[test]
  fn project_dynamic_must_not_duplicate_fields() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      dynamic = ["version", "version"]
      "#
    })
    .error(Message {
      range: (2, 22, 2, 31),
      text: "`project.dynamic` contains duplicate field `version`",
    })
    .run();
  }

  #[test]
  fn project_dynamic_must_not_conflict_with_static_values() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      dynamic = ["version", "description"]
      description = "demo package"
      "#
    })
    .error(Message {
      range: (3, 11, 3, 20),
      text: "`project.dynamic` field `version` must not also be provided statically",
    })
    .error(Message {
      range: (3, 22, 3, 35),
      text: "`project.dynamic` field `description` must not also be provided statically",
    })
    .run();
  }

  #[test]
  fn project_import_names_must_be_array_of_strings() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      import-names = "demo"
      "#
    })
    .error(Message {
      range: (3, 15, 3, 21),
      text: "`project.import-names` must be an array of strings",
    })
    .run();
  }

  #[test]
  fn project_import_names_items_must_be_strings() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      import-names = [1]
      "#
    })
    .error(Message {
      range: (3, 16, 3, 17),
      text: "`project.import-names` items must be strings",
    })
    .run();
  }

  #[test]
  fn project_import_names_detects_duplicates_across_fields() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      import-names = ["demo"]
      import-namespaces = ["demo; python_version < '4'"]
      "#
    })
    .error(Message {
      range: (4, 21, 4, 49),
      text: "duplicated names are not allowed in `project.import-names`/`project.import-namespaces` (found `demo`)",
    })
    .run();
  }

  #[test]
  fn project_import_names_require_parent_namespaces() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      import-names = ["demo.core.utils"]
      "#
    })
    .error(Message {
      range: (3, 16, 3, 33),
      text: "`demo.core.utils` is missing parent namespace `demo`; all parents must be listed in `project.import-names`/`project.import-namespaces`",
    })
    .run();
  }

  #[test]
  fn project_reports_unknown_keys() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      custom = "value"
      "#
    })
    .error(Message {
      range: (3, 0, 3, 6),
      text: "`project.custom` is not defined by PEP 621; move custom settings under `[tool]` or another accepted PEP section",
    })
    .run();
  }

  #[test]
  fn dependency_groups_include_group_must_exist() {
    Test::new(indoc! {
      r#"
      [dependency-groups]
      test = [{ include-group = "lint" }]
      "#
    })
    .error(Message {
      range: (1, 26, 1, 32),
      text: "`dependency-groups.test` includes unknown group `lint`",
    })
    .run();
  }

  #[test]
  fn dependency_groups_include_group_must_be_string() {
    Test::new(indoc! {
      r"
      [dependency-groups]
      test = [{ include-group = 1 }]
      "
    })
    .error(Message {
      range: (1, 26, 1, 27),
      text: "`include-group` value must be a string",
    })
    .run();
  }

  #[test]
  fn dependency_groups_include_group_normalizes_names() {
    Test::new(indoc! {
      r#"
      [dependency-groups]
      "Lint.Group" = ["ruff"]
      test = [{ include-group = "lint_group" }]
      "#
    })
    .run();
  }

  #[test]
  fn dependency_groups_include_group_must_be_only_key() {
    Test::new(indoc! {
      r#"
      [dependency-groups]
      test = [{ include-group = "lint", extra = true }]
      "#
    })
    .error(Message {
      range: (1, 26, 1, 32),
      text: "`include-group` objects must contain only the `include-group` key",
    })
    .run();
  }

  #[test]
  fn dependency_groups_include_group_is_defined() {
    Test::new(indoc! {
      r#"
      [dependency-groups]
      lint = ["ruff"]
      test = [{ include-group = "lint" }]
      "#
    })
    .run();
  }

  #[test]
  fn project_license_table_requires_file_or_text() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = { }
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = { file = "LICENSE", text = "Apache" }
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = { file = 1 }
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = { text = 1 }
      "#
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
  #[cfg(unix)]
  fn project_license_table_file_path_must_be_relative_unix() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = { file = "/LICENSE" }
      "#
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
  #[cfg(windows)]
  fn project_license_table_file_path_must_be_relative_windows() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = { file = "/LICENSE" }
      "#
    })
    .warning(Message {
      range: (3, 10, 3, 31),
      text: "`project.license` tables are deprecated; prefer a SPDX expression string and `project.license-files`",
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = { file = "LICENSE" }
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = { file = "LICENSE" }
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = ""
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = []
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "Apache-2.0 OR NotARealLicense"
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "mit"
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "wxWindows"
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "MIT"
      classifiers = [
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python",
      ]
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      classifiers = ["License :: OSI Approved :: MIT License"]
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "MIT"
      license-files = "LICENSE*"
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "MIT"
      license-files = [1]
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "MIT"
      license-files = ["/LICENSE"]
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "MIT"
      license-files = ["..\\LICENSE"]
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "MIT"
      license-files = ["LICENSE*"]"#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "MIT"
      license-files = []
      "#
    })
    .run();
  }

  #[test]
  fn project_license_files_must_point_to_existing_utf8_files() {
    Test::with_tempdir(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "MIT"
      license-files = ["LICENSE"]
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "MIT"
      license-files = ["LICENSE"]
      "#
    })
    .write_file("LICENSE", "MIT")
    .run();
  }

  #[test]
  fn project_license_files_accepts_nested_license_path() {
    Test::with_tempdir(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "MIT"
      license-files = ["licenses/LICENSE"]
      "#
    })
    .write_file("licenses/LICENSE", "MIT")
    .run();
  }

  #[test]
  fn project_license_files_supports_globstar_patterns() {
    Test::with_tempdir(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = "MIT"
      license-files = ["licenses/**/LICENSE"]
      "#
    })
    .write_file("licenses/nested/deeper/LICENSE", "MIT")
    .run();
  }

  #[test]
  fn project_license_files_requires_string_license_when_present() {
    Test::with_tempdir(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      license = { file = "LICENSE" }
      license-files = ["LICENSE"]
      "#
    })
    .write_file("LICENSE", "MIT")
    .error(Message {
      range: (3, 10, 3, 30),
      text: "`project.license` must be a string SPDX expression when `project.license-files` is present",
    })
    .run();
  }

  #[test]
  fn project_urls_must_be_a_table() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      urls = "https://example.com"
      "#
    })
    .error(Message {
      range: (3, 7, 3, 28),
      text: "`project.urls` must be a table of string URLs",
    })
    .run();
  }

  #[test]
  fn project_urls_entries_must_be_strings() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      urls = { homepage = 123 }
      "#
    })
    .error(Message {
      range: (3, 20, 3, 23),
      text: "`project.urls` entry `homepage` must be a string URL",
    })
    .run();
  }

  #[test]
  fn project_urls_entries_must_not_be_empty() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      urls = { homepage = "" }
      "#
    })
    .error(Message {
      range: (3, 20, 3, 22),
      text: "`project.urls` entry `homepage` must not be empty",
    })
    .run();
  }

  #[test]
  fn project_urls_entries_must_be_valid_urls() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      urls = { homepage = "example.com" }
      "#
    })
    .error(Message {
      range: (3, 20, 3, 33),
      text: "`project.urls` entry `homepage` must be a valid URL: relative URL without a base",
    })
    .run();
  }

  #[test]
  fn project_urls_entries_must_use_http_or_https() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      urls = { homepage = "ftp://example.com" }
      "#
    })
    .error(Message {
      range: (3, 20, 3, 39),
      text: "`project.urls` entry `homepage` must use an `http` or `https` URL",
    })
    .run();
  }

  #[test]
  fn project_urls_labels_must_not_exceed_limit() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [project.urls]
      this-label-is-way-too-long-to-be-valid = "https://example.com"
      "#
    })
    .error(Message {
      range: (5, 0, 5, 38),
      text: "`project.urls` label `this-label-is-way-too-long-to-be-valid` must be 32 characters or fewer",
    })
    .run();
  }

  #[test]
  fn poetry_urls_must_be_a_table() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [tool.poetry]
      name = "demo"
      version = "1.0.0"
      urls = "https://example.com"
      "#
    })
    .error(Message {
      range: (7, 0, 7, 28),
      text: "expected object for `tool.poetry.urls`, got string \"https://example.com\"",
    })
    .run();
  }

  #[test]
  fn flit_urls_entries_must_be_valid_urls() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [tool.flit.metadata.urls]
      Homepage = "example.com"
      "#
    })
    .error(Message {
      range: (5, 11, 5, 24),
      text: "`tool.flit.metadata.urls` entry `Homepage` must be a valid URL: relative URL without a base",
    })
    .run();
  }

  #[test]
  fn setuptools_project_urls_entries_must_not_be_empty() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [tool.setuptools]
      project_urls = { Homepage = "" }
      "#
    })
    .error(Message {
      range: (5, 0, 5, 32),
      text: "unknown setting `tool.setuptools.project_urls`",
    })
    .run();
  }

  #[test]
  fn project_readme_string_must_point_to_existing_file() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = "README.md"
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = "README.txt"
      "#
    })
    .write_file("README.txt", "# readme")
    .error(Message {
      range: (3, 9, 3, 21),
      text: "`project.readme` must point to a `.md` or `.rst` file when specified as a string",
    })
    .run();
  }

  #[test]
  #[cfg(unix)]
  fn project_readme_string_path_must_be_relative_unix() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = "/README.md"
      "#
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
  #[cfg(windows)]
  fn project_readme_string_path_must_be_relative_windows() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = "/README.md"
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = { file = "README.md" }
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = { content-type = "text/markdown" }
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = { file = "README.md", text = "inline", content-type = "text/markdown" }
      "#
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
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = { text = 1, content-type = "text/markdown" }
      "#
    })
    .error(Message {
      range: (3, 18, 3, 19),
      text: "`project.readme.text` must be a string",
    })
    .run();
  }

  #[test]
  fn project_readme_table_requires_supported_content_type() {
    Test::with_tempdir(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = { file = "README.md", content-type = "text/html" }
      "#
    })
    .write_file("README.md", "# readme")
    .error(Message {
      range: (3, 46, 3, 57),
      text: "`project.readme.content-type` must be one of `text/markdown`, `text/x-rst`, or `text/plain`",
    })
    .run();
  }

  #[test]
  fn project_readme_table_file_must_exist() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = { file = "README.md", content-type = "text/markdown" }
      "#
    })
    .error(Message {
      range: (3, 18, 3, 29),
      text: "file `README.md` for `project.readme` does not exist",
    })
    .run();
  }

  #[test]
  fn project_readme_table_accepts_text_plain() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = { text = "inline", content-type = "text/plain" }
      "#
    })
    .warning(Message {
      range: (3, 43, 3, 55),
      text: "`project.readme.content-type` is `text/plain`; consider `text/markdown` or `text/x-rst` for better rendering on package indexes",
    })
    .run();
  }

  #[test]
  fn valid_project_readme_table_with_content_type_and_file() {
    Test::with_tempdir(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      readme = { file = "README.md", content-type = "text/markdown" }
      "#
    })
    .write_file("README.md", "# readme")
    .run();
  }

  #[test]
  fn project_entry_points_rejects_console_scripts_group() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [project.entry-points.console_scripts]
      cli = "demo:main"
      "#
    })
    .error(Message {
      range: (4, 22, 4, 37),
      text: "`project.entry-points.console_scripts` is not allowed; use `[project.scripts]` instead",
    })
    .run();
  }

  #[test]
  fn project_entry_points_rejects_nested_entry_point_tables() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [project.entry-points.my_group]
      nested.table = "demo:main"
      "#
    })
    .error(Message {
      range: (5, 0, 5, 6),
      text: "`project.entry-points.my_group.nested` must be a string object reference; entry point groups cannot be nested",
    })
    .run();
  }

  #[test]
  fn project_entry_points_requires_table() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"
      entry-points = "demo:main"
      "#
    })
    .error(Message {
      range: (3, 15, 3, 26),
      text: "`project.entry-points` must be a table of entry point groups",
    })
    .run();
  }

  #[test]
  fn project_entry_points_group_names_must_match_pattern() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [project.entry-points."bad group"]
      cli = "demo:main"
      "#
    })
    .error(Message {
      range: (4, 22, 4, 33),
      text: "`project.entry-points` group names must match `^\\w+(\\.\\w+)*$`",
    })
    .run();
  }

  #[test]
  fn project_entry_point_names_must_not_have_invalid_characters() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [project.scripts]
      " bad" = "json:loads"
      "#
    })
    .error(Message {
      range: (5, 0, 5, 6),
      text: "`project.scripts. bad` name must not start or end with whitespace",
    })
    .run();
  }

  #[test]
  fn project_entry_point_values_must_reference_importable_objects() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [project.scripts]
      cli = "1demo:main"
      "#
    })
    .error(Message {
      range: (5, 6, 5, 18),
      text: "`project.scripts.cli` must reference an importable module path (e.g. `package.module`) optionally followed by `:qualname`",
    })
    .run();
  }

  #[test]
  fn project_entry_point_targets_must_be_importable() {
    Test::with_tempdir(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [project.scripts]
      cli = "doesnotexist:main"
      "#
    })
    .error(Message {
      range: (5, 6, 5, 25),
      text: "`project.scripts.cli` target `doesnotexist:main` is not importable: ModuleNotFoundError: No module named 'doesnotexist'",
    })
    .run();
  }

  #[test]
  fn project_entry_point_targets_importable_ok() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [project.scripts]
      cli = "json:loads"
      "#
    })
    .run();
  }

  #[test]
  fn project_entry_points_warn_when_cwd_needed() {
    Test::with_tempdir(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [project.scripts]
      cli = "demo:main"
      "#
    })
    .write_file("demo/__init__.py", "def main():\n    return 0\n")
    .warning(Message {
      range: (5, 6, 5, 17),
      text: "`project.scripts.cli` target `demo:main` is not importable in isolated mode (without the current working directory on `sys.path`): ModuleNotFoundError: No module named 'demo'",
    })
    .run();
  }

  #[test]
  fn json_schema_reports_additional_tool_properties() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [tool.black]
      unknown = true
      "#
    })
    .error(Message {
      range: (5, 0, 5, 14),
      text: "unknown setting `tool.black.unknown`",
    })
    .run();
  }

  #[test]
  fn json_schema_reports_tool_type_mismatches() {
    Test::new(indoc! {
      r#"
      [project]
      name = "demo"
      version = "1.0.0"

      [tool.black]
      line-length = "eighty"
      "#
    })
    .error(Message {
      range: (5, 0, 5, 22),
      text: "expected integer for `tool.black.line-length`, got string \"eighty\"",
    })
    .run();
  }
}
