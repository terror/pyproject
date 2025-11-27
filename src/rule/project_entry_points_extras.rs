use super::*;

pub(crate) struct ProjectEntryPointsExtrasRule;

impl Rule for ProjectEntryPointsExtrasRule {
  fn id(&self) -> &'static str {
    "project-entry-points-extras"
  }

  fn message(&self) -> &'static str {
    "extras in entry point definitions are deprecated"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let document = context.document();

    let mut diagnostics = Vec::new();

    if let Some(scripts) = context.get("project.scripts") {
      diagnostics.extend(Self::scan_scripts_table(
        document,
        "project.scripts",
        &scripts,
      ));
    }

    if let Some(gui_scripts) = context.get("project.gui-scripts") {
      diagnostics.extend(Self::scan_scripts_table(
        document,
        "project.gui-scripts",
        &gui_scripts,
      ));
    }

    if let Some(entry_points) = context.get("project.entry-points") {
      diagnostics
        .extend(Self::scan_entry_points_table(document, &entry_points));
    }

    diagnostics
  }
}

impl ProjectEntryPointsExtrasRule {
  fn has_extras(raw: &str) -> bool {
    let trimmed = raw.trim();
    trimmed.split_once('[').is_some()
  }

  fn scan_entry_points_table(
    document: &Document,
    entry_points: &Node,
  ) -> Vec<Diagnostic> {
    let Some(table) = entry_points.as_table() else {
      return Vec::new();
    };

    let mut diagnostics = Vec::new();

    for (group_key, group) in table.entries().read().iter() {
      let Some(group_table) = group.as_table() else {
        continue;
      };

      for (entry_key, entry_value) in group_table.entries().read().iter() {
        let location = format!(
          "project.entry-points.{}.{}",
          group_key.value(),
          entry_key.value()
        );

        diagnostics.extend(Self::scan_value(document, &location, entry_value));
      }
    }

    diagnostics
  }

  fn scan_scripts_table(
    document: &Document,
    field: &str,
    node: &Node,
  ) -> Vec<Diagnostic> {
    let Some(table) = node.as_table() else {
      return Vec::new();
    };

    let mut diagnostics = Vec::new();

    for (key, value) in table.entries().read().iter() {
      let location = format!("{field}.{}", key.value());
      diagnostics.extend(Self::scan_value(document, &location, value));
    }

    diagnostics
  }

  fn scan_value(
    document: &Document,
    location: &str,
    value: &Node,
  ) -> Vec<Diagnostic> {
    let Some(string) = value.as_str() else {
      return Vec::new();
    };

    let raw = string.value();

    if raw.trim().is_empty() || !Self::has_extras(raw) {
      return Vec::new();
    }

    vec![Diagnostic::warning(
      format!(
        "`{location}` uses extras in entry point definitions; extras are deprecated for entry points and may be ignored by consumers"
      ),
      value.span(&document.content),
    )]
  }
}
