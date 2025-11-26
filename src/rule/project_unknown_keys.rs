use super::*;

pub(crate) struct ProjectUnknownKeysRule;

impl Rule for ProjectUnknownKeysRule {
  fn display(&self) -> &'static str {
    "project table contains unknown keys"
  }

  fn id(&self) -> &'static str {
    "project-unknown-keys"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(project) = context.project() else {
      return Vec::new();
    };

    let Some(table) = project.as_table() else {
      return Vec::new();
    };

    let document = context.document();

    table
      .entries()
      .read()
      .iter()
      .filter_map(|(key, _)| Self::diagnostic_for_key(document, key))
      .collect()
  }
}

impl ProjectUnknownKeysRule {
  fn diagnostic_for_key(document: &Document, key: &Key) -> Option<Diagnostic> {
    let name = key.value();

    if Self::is_allowed(name) {
      return None;
    }

    Some(Diagnostic::error(
      format!(
        "`project.{name}` is not defined by PEP 621; move custom settings under `[tool]` or another accepted PEP section"
      ),
      key.span(&document.content),
    ))
  }

  fn is_allowed(key: &str) -> bool {
    // PEP 621 core metadata keys.
    matches!(
      key,
      "authors"
        | "classifiers"
        | "dependencies"
        | "description"
        | "dynamic"
        | "entry-points"
        | "gui-scripts"
        | "keywords"
        | "license"
        | "maintainers"
        | "name"
        | "optional-dependencies"
        | "readme"
        | "requires-python"
        | "scripts"
        | "urls"
        | "version"
    ) ||
    // Accepted extensions defined outside of PEP 621.
    matches!(
      key,
      "import-names" | "import-namespaces" | "license-files"
    )
  }
}
