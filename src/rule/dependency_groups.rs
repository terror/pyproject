use super::*;

define_rule! {
  /// Validates `dependency-groups` configuration per PEP 735.
  ///
  /// Checks that `include-group` objects contain only the `include-group` key
  /// and that referenced groups exist in the dependency-groups table.
  DependencyGroupsRule {
    id: "dependency-groups",
    message: "invalid `dependency-groups` configuration",
    run(context) {
      let Some(groups) = context.get("dependency-groups") else {
        return Vec::new();
      };

      let groups_table = match groups.as_table() {
        Some(table) if table.kind() == TableKind::Regular => table,
        _ => {
          return vec![Diagnostic::error(
            "`dependency-groups` must be a table",
            groups.span(context.content()),
          )];
        }
      };

      let mut diagnostics = Vec::new();
      let mut groups = HashMap::<String, DependencyGroup>::new();
      let mut all_groups = Vec::new();
      let mut group_names = Vec::new();

      for (group_key, group_value) in groups_table.entries().read().iter() {
        let group_name = group_key.value();
        let normalized_name = Self::normalize_group_name(group_name);

        if !PROJECT_NAME.is_match(group_name) {
          diagnostics.push(Diagnostic::error(
            format!(
              "`dependency-groups` group name `{group_name}` must be a valid non-normalized name"
            ),
            group_key.span(context.content()),
          ));
        }

        let includes = Self::validate_group(
          context,
          group_name,
          group_value,
          &mut diagnostics,
        );

        let group = DependencyGroup {
          name: group_name.to_string(),
          includes,
        };

        all_groups.push(group.clone());

        if let Some(existing) = groups.get(&normalized_name) {
          diagnostics.push(Diagnostic::error(
            format!(
              "`dependency-groups` contains duplicate group names after normalization: `{}` and `{group_name}`",
              existing.name
            ),
            group_key.span(context.content()),
          ));

          continue;
        }

        group_names.push(normalized_name.clone());
        groups.insert(normalized_name, group);
      }

      for group in &all_groups {
        for include in &group.includes {
          if groups.contains_key(&include.normalized_name) {
            continue;
          }

          diagnostics.push(Diagnostic::error(
            format!(
              "`dependency-groups.{}` includes unknown group `{}`",
              group.name, include.name
            ),
            include.range,
          ));
        }
      }

      diagnostics.extend(Self::find_cycles(&groups, &group_names));

      diagnostics
    }
  }
}

#[derive(Clone)]
struct DependencyGroup {
  includes: Vec<Include>,
  name: String,
}

#[derive(Clone)]
struct Include {
  name: String,
  normalized_name: String,
  range: lsp::Range,
}

impl DependencyGroupsRule {
  fn find_cycles(
    groups: &HashMap<String, DependencyGroup>,
    group_names: &[String],
  ) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut path = Vec::new();
    let mut visited = HashSet::new();

    for group_name in group_names {
      Self::visit_group(
        groups,
        group_name,
        &mut path,
        &mut visited,
        &mut diagnostics,
      );
    }

    diagnostics
  }

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

  fn validate_group(
    context: &RuleContext,
    group_name: &str,
    group_value: &Node,
    diagnostics: &mut Vec<Diagnostic>,
  ) -> Vec<Include> {
    let location = format!("dependency-groups.{group_name}");

    let Some(array) = group_value.as_array() else {
      diagnostics.push(Diagnostic::error(
        format!("`{location}` must be an array"),
        group_value.span(context.content()),
      ));

      return Vec::new();
    };

    let mut includes = Vec::new();

    for (index, item) in array.items().read().iter().enumerate() {
      let item_location = format!("{location}[{index}]");

      if let Some(string) = item.as_str() {
        let value = string.value();

        if let Err(error) = Requirement::<VerbatimUrl>::from_str(value) {
          diagnostics.push(Diagnostic::error(
            format!(
              "`{item_location}` item `{value}` is not a valid PEP 508 dependency: {}",
              error.message.to_string().to_lowercase()
            ),
            item.span(context.content()),
          ));
        }

        continue;
      }

      let Some(table) = item.as_table() else {
        diagnostics.push(Diagnostic::error(
          format!(
            "`{item_location}` must be a PEP 508 dependency string or an `include-group` object"
          ),
          item.span(context.content()),
        ));

        continue;
      };

      if table.kind() != TableKind::Inline {
        diagnostics.push(Diagnostic::error(
          format!(
            "`{item_location}` must be a PEP 508 dependency string or an `include-group` object"
          ),
          item.span(context.content()),
        ));

        continue;
      }

      let entries = table.entries().read();

      if entries.len() != 1 {
        let range = entries
          .iter()
          .find(|(key, _)| key.value() == "include-group")
          .map_or_else(
            || item.span(context.content()),
            |(_, value)| value.span(context.content()),
          );

        diagnostics.push(Diagnostic::error(
          "`include-group` objects must contain only the `include-group` key",
          range,
        ));

        continue;
      }

      let Some((include_key, include_group)) = entries.iter().next() else {
        continue;
      };

      if include_key.value() != "include-group" {
        diagnostics.push(Diagnostic::error(
          "`dependency-groups` include objects must use the `include-group` key",
          include_key.span(context.content()),
        ));

        continue;
      }

      let Some(value) = include_group.as_str() else {
        diagnostics.push(Diagnostic::error(
          "`include-group` value must be a string",
          include_group.span(context.content()),
        ));

        continue;
      };

      let name = value.value();

      if !PROJECT_NAME.is_match(name) {
        diagnostics.push(Diagnostic::error(
          format!(
            "`{item_location}` include target `{name}` must be a valid non-normalized name"
          ),
          include_group.span(context.content()),
        ));

        continue;
      }

      includes.push(Include {
        name: name.to_string(),
        normalized_name: Self::normalize_group_name(name),
        range: include_group.span(context.content()),
      });
    }

    includes
  }

  fn visit_group(
    groups: &HashMap<String, DependencyGroup>,
    group_name: &str,
    path: &mut Vec<String>,
    visited: &mut HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
  ) {
    if visited.contains(group_name) {
      return;
    }

    let Some(group) = groups.get(group_name) else {
      return;
    };

    path.push(group_name.to_string());

    for include in &group.includes {
      if let Some(index) = path
        .iter()
        .position(|name| name == &include.normalized_name)
      {
        let mut cycle = path[index..]
          .iter()
          .filter_map(|name| groups.get(name).map(|group| group.name.clone()))
          .collect::<Vec<_>>();

        if let Some(group) = groups.get(&include.normalized_name) {
          cycle.push(group.name.clone());
        }

        diagnostics.push(Diagnostic::error(
          format!("cyclic dependency group include: {}", cycle.join(" -> ")),
          include.range,
        ));

        continue;
      }

      Self::visit_group(
        groups,
        &include.normalized_name,
        path,
        visited,
        diagnostics,
      );
    }

    path.pop();
    visited.insert(group_name.to_string());
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
