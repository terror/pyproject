use super::*;

pub(crate) struct RuleContext<'a> {
  document: &'a Document,
}

impl<'a> RuleContext<'a> {
  pub(crate) fn document(&self) -> &Document {
    self.document
  }

  /// Extract the package name from a PEP 508 dependency string.
  ///
  /// This extracts the raw package name before any normalization,
  /// which is useful for checking if the name needs to be normalized.
  pub(crate) fn extract_dependency_name(value: &str) -> Option<&str> {
    let trimmed = value.trim_start();

    let end = trimmed
      .find([' ', '\t', '[', '(', '!', '=', '<', '>', '~', ';', '@', ','])
      .unwrap_or(trimmed.len());

    let name = trimmed[..end].trim_end();

    (!name.is_empty()).then_some(name)
  }

  /// Get a node from the document using a dot-separated path.
  ///
  /// This method navigates through the TOML document structure using a path string
  /// where each segment is separated by a dot (`.`). It returns `Some(Node)` if the
  /// path exists, or `None` if the path is invalid or doesn't exist.
  pub(crate) fn get(&self, path: &str) -> Option<Node> {
    let mut current = self.document.tree.clone().into_dom();

    if path.is_empty() {
      return Some(current);
    }

    for key in path.split('.') {
      if key.is_empty() {
        return None;
      }

      let Ok(next) = current.try_get(key) else {
        return None;
      };

      current = next;
    }

    Some(current)
  }

  pub(crate) fn new(document: &'a Document) -> Self {
    Self { document }
  }

  pub(crate) fn project(&self) -> Option<Node> {
    self.get("project")
  }

  pub(crate) fn tree(&self) -> &Parse {
    &self.document.tree
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extract_dependency_name_simple_package() {
    assert_eq!(
      RuleContext::extract_dependency_name("requests"),
      Some("requests")
    );
  }

  #[test]
  fn extract_dependency_name_with_version_specifier() {
    assert_eq!(
      RuleContext::extract_dependency_name("requests>=2.0.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_dependency_name_with_exact_version() {
    assert_eq!(
      RuleContext::extract_dependency_name("requests==2.28.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_dependency_name_with_extras() {
    assert_eq!(
      RuleContext::extract_dependency_name("requests[security]>=2.0.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_dependency_name_with_environment_marker() {
    assert_eq!(
      RuleContext::extract_dependency_name(
        "requests>=2.0.0; python_version >= '3.8'"
      ),
      Some("requests")
    );
  }

  #[test]
  fn extract_dependency_name_with_url() {
    assert_eq!(
      RuleContext::extract_dependency_name(
        "package @ https://example.com/package.tar.gz"
      ),
      Some("package")
    );
  }

  #[test]
  fn extract_dependency_name_with_leading_whitespace() {
    assert_eq!(
      RuleContext::extract_dependency_name("  requests>=2.0.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_dependency_name_with_trailing_whitespace_before_version() {
    assert_eq!(
      RuleContext::extract_dependency_name("requests >=2.0.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_dependency_name_with_comma() {
    assert_eq!(
      RuleContext::extract_dependency_name("requests>=2.0.0,<3.0.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_dependency_name_with_tilde_equal() {
    assert_eq!(
      RuleContext::extract_dependency_name("requests~=2.28.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_dependency_name_with_not_equal() {
    assert_eq!(
      RuleContext::extract_dependency_name("requests!=2.27.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_dependency_name_empty_string() {
    assert_eq!(RuleContext::extract_dependency_name(""), None);
  }

  #[test]
  fn extract_dependency_name_only_whitespace() {
    assert_eq!(RuleContext::extract_dependency_name("   "), None);
  }

  #[test]
  fn extract_dependency_name_with_parentheses() {
    assert_eq!(
      RuleContext::extract_dependency_name("requests (>=2.0.0)"),
      Some("requests")
    );
  }

  #[test]
  fn get_returns_root_for_empty_path() {
    let document = Document::from(indoc! {
      r#"
      [project]
      name = "demo"
      "#
    });

    let context = RuleContext::new(&document);

    let root = context.get("").unwrap();

    match root {
      Node::Table(table) => assert!(table.get("project").is_some()),
      other => panic!("expected document root to be a table, got {other:?}"),
    }
  }

  #[test]
  fn get_returns_nested_value() {
    let document = Document::from(indoc! {
      r#"
      [project]
      name = "demo"
      description = "example"
      "#
    });

    let context = RuleContext::new(&document);

    let name = context.get("project.name").unwrap();

    match name {
      Node::Str(value) => assert_eq!(value.value(), "demo"),
      other => panic!("expected project.name to be a string, got {other:?}"),
    }
  }

  #[test]
  fn get_rejects_invalid_paths() {
    let document = Document::from(indoc! {
      r#"
      [project]
      name = "demo"
      "#
    });

    let context = RuleContext::new(&document);

    let cases = &[
      "nonexistent",
      "project.",
      "project.name.extra",
      "project.nonexistent",
    ];

    for case in cases {
      assert!(
        context.get(case).is_none(),
        "expected path '{case}' to be invalid"
      );
    }
  }
}
