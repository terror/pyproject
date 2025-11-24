use super::*;

pub(crate) struct DependencyGroupsRule;

impl Rule for DependencyGroupsRule {
  fn header(&self) -> &'static str {
    "dependency-groups configuration issues"
  }

  fn id(&self) -> &'static str {
    "dependency-groups"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let Some(groups) = context.get("dependency-groups") else {
      return Vec::new();
    };

    let document = context.document();

    let Some(groups_table) = groups.as_table() else {
      return Vec::new();
    };

    let group_names = groups_table
      .entries()
      .read()
      .iter()
      .map(|(key, _)| Self::normalize_group_name(key.value()))
      .collect::<HashSet<String>>();

    let mut diagnostics = Vec::new();

    for (group_key, group_value) in groups_table.entries().read().iter() {
      let Some(array) = group_value.as_array() else {
        continue;
      };

      for item in array.items().read().iter() {
        let Some(table) = item.as_table() else {
          continue;
        };

        let entries = table.entries().read();

        if entries.len() != 1 {
          let range = entries
            .iter()
            .find(|(key, _)| key.value() == "include-group")
            .map_or_else(
              || item.range(&document.content),
              |(_, value)| value.range(&document.content),
            );

          diagnostics.push(Diagnostic::new(
            "`include-group` objects must contain only the `include-group` key",
            range,
            lsp::DiagnosticSeverity::ERROR,
          ));

          continue;
        }

        let (include_key, include_group) = entries.iter().next().unwrap();

        if include_key.value() != "include-group" {
          diagnostics.push(Diagnostic::new(
            "`dependency-groups` include objects must use the `include-group` key",
            include_key.range(&document.content),
            lsp::DiagnosticSeverity::ERROR,
          ));

          continue;
        }

        let Some(value) = include_group.as_str() else {
          diagnostics.push(Diagnostic::new(
            "`include-group` value must be a string",
            include_group.range(&document.content),
            lsp::DiagnosticSeverity::ERROR,
          ));

          continue;
        };

        let name = value.value();

        if group_names.contains(&Self::normalize_group_name(name)) {
          continue;
        }

        diagnostics.push(Diagnostic::new(
          format!(
            "`dependency-groups.{}` includes unknown group `{}`",
            group_key.value(),
            name
          ),
          include_group.range(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        ));
      }
    }

    diagnostics
  }
}

impl DependencyGroupsRule {
  fn normalize_group_name(name: &str) -> String {
    let mut normalized = String::new();

    let mut last_was_sep = false;

    for ch in name.chars() {
      let is_sep = matches!(ch, '-' | '_' | '.');

      if is_sep {
        if !last_was_sep {
          normalized.push('-');
        }

        last_was_sep = true;

        continue;
      }

      normalized.push(ch.to_ascii_lowercase());

      last_was_sep = false;
    }

    normalized
  }
}

#[cfg(test)]
mod tests {
  use {super::*, pretty_assertions::assert_eq};

  #[test]
  fn normalizes_case_and_hyphenates() {
    assert_eq!(
      DependencyGroupsRule::normalize_group_name("Feature-Flags"),
      "feature-flags"
    );
  }

  #[test]
  fn replaces_underscores_and_dots() {
    assert_eq!(
      DependencyGroupsRule::normalize_group_name("data_access.layer"),
      "data-access-layer"
    );
  }

  #[test]
  fn collapses_adjacent_separators() {
    assert_eq!(
      DependencyGroupsRule::normalize_group_name("core__api--v2..beta"),
      "core-api-v2-beta"
    );
  }

  #[test]
  fn preserves_single_leading_separator() {
    assert_eq!(
      DependencyGroupsRule::normalize_group_name("-Experimental_Feature"),
      "-experimental-feature"
    );
  }
}
