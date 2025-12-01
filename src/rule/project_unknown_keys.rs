use super::*;

define_rule! {
  /// Errors when `project` table contains keys not defined by PEP 621.
  ///
  /// Custom settings should be placed under `[tool]` or other accepted sections.
  ProjectUnknownKeysRule {
    id: "project-unknown-keys",
    message: "project table contains unknown keys",
    run(context) {
      let Some(project) = context.get("project") else {
        return Vec::new();
      };

      let Some(table) = project.as_table() else {
        return Vec::new();
      };

      table
        .entries()
        .read()
        .iter()
        .filter_map(|(key, _)| Self::diagnostic_for_key(context.content(), key))
        .collect()
    }
  }
}

impl ProjectUnknownKeysRule {
  fn diagnostic_for_key(content: &Rope, key: &Key) -> Option<Diagnostic> {
    let name = key.value();

    if Self::is_allowed(name) {
      return None;
    }

    Some(Diagnostic::error(
      format!(
        "`project.{name}` is not defined by PEP 621; move custom settings under `[tool]` or another accepted PEP section"
      ),
      key.span(content),
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
