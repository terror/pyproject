use super::*;

define_rule! {
  ProjectImportNamesRule {
    id: "project-import-names",
    message: "invalid `project.import-names` / `project.import-namespaces` configuration",
    run(context) {
      let content = context.content();

      let mut diagnostics = Vec::new();

      let mut entries = Vec::new();

      if let Some(import_names) = context.get("project.import-names") {
        Self::collect_entries(
          content,
          "project.import-names",
          import_names,
          &mut diagnostics,
          &mut entries,
        );
      }

      if let Some(import_namespaces) = context.get("project.import-namespaces")
      {
        Self::collect_entries(
          content,
          "project.import-namespaces",
          import_namespaces,
          &mut diagnostics,
          &mut entries,
        );
      }

      if entries.is_empty() {
        return diagnostics;
      }

      let mut seen = HashSet::new();

      for (name, node) in &entries {
        if !seen.insert(name.clone()) {
          diagnostics.push(Self::duplicate_name_diagnostic(
            content, node, name,
          ));
        }
      }

      let available: HashSet<String> =
        entries.iter().map(|(name, _)| name.clone()).collect();

      for (name, node) in &entries {
        for parent in Self::parent_names(name) {
          if !available.contains(&parent) {
            diagnostics.push(Self::missing_parent_diagnostic(
              content, node, name, &parent,
            ));

            break;
          }
        }
      }

      diagnostics
    }
  }
}

impl ProjectImportNamesRule {
  fn collect_entries(
    content: &Rope,
    field: &'static str,
    node: Node,
    diagnostics: &mut Vec<Diagnostic>,
    entries: &mut Vec<(String, Node)>,
  ) {
    let Some(array) = node.as_array() else {
      diagnostics.push(Diagnostic::error(
        format!("`{field}` must be an array of strings"),
        node.span(content),
      ));

      return;
    };

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        diagnostics.push(Diagnostic::error(
          format!("`{field}` items must be strings"),
          item.span(content),
        ));

        continue;
      };

      let sanitized = Self::sanitize_name(string.value());

      entries.push((sanitized.to_string(), item.clone()));
    }
  }

  fn duplicate_name_diagnostic(
    content: &Rope,
    node: &Node,
    name: &str,
  ) -> Diagnostic {
    Diagnostic::error(
      format!(
        "duplicated names are not allowed in `project.import-names`/`project.import-namespaces` (found `{name}`)"
      ),
      node.span(content),
    )
  }

  fn missing_parent_diagnostic(
    content: &Rope,
    node: &Node,
    name: &str,
    parent: &str,
  ) -> Diagnostic {
    Diagnostic::error(
      format!(
        "`{name}` is missing parent namespace `{parent}`; all parents must be listed in `project.import-names`/`project.import-namespaces`"
      ),
      node.span(content),
    )
  }

  fn parent_names(name: &str) -> Vec<String> {
    let mut parents = Vec::new();

    let mut current = String::new();

    let mut segments = name.split('.').peekable();

    while let Some(segment) = segments.next() {
      if segments.peek().is_none() {
        break;
      }

      if !current.is_empty() {
        current.push('.');
      }

      current.push_str(segment);

      if !current.is_empty() {
        parents.push(current.clone());
      }
    }

    parents
  }

  fn sanitize_name(raw: &str) -> &str {
    raw.split_once(';').map_or(raw, |(name, _)| name).trim_end()
  }
}

#[cfg(test)]
mod tests {
  use {super::*, pretty_assertions::assert_eq};

  #[test]
  fn sanitize_name_strips_markers_and_trailing_whitespace() {
    assert_eq!(
      ProjectImportNamesRule::sanitize_name(
        "demo.sub  ; python_version>='3.11'"
      ),
      "demo.sub"
    );

    assert_eq!(
      ProjectImportNamesRule::sanitize_name("demo.sub  "),
      "demo.sub"
    );
  }

  #[test]
  fn parent_names_builds_all_namespaces() {
    assert_eq!(
      ProjectImportNamesRule::parent_names("foo.bar.baz"),
      vec!["foo".to_string(), "foo.bar".to_string()]
    );

    assert!(ProjectImportNamesRule::parent_names("foo").is_empty());
  }
}
