use super::*;

pub(crate) struct ProjectEntryPointsRule;

impl Rule for ProjectEntryPointsRule {
  fn header(&self) -> &'static str {
    "invalid project entry points configuration"
  }

  fn id(&self) -> &'static str {
    "project-entry-points"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let document = context.document();

    let mut diagnostics = Vec::new();

    if let Some(scripts) = context.get("project.scripts") {
      diagnostics.extend(Self::validate_scripts_table(
        document,
        "project.scripts",
        &scripts,
      ));
    }

    if let Some(gui_scripts) = context.get("project.gui-scripts") {
      diagnostics.extend(Self::validate_scripts_table(
        document,
        "project.gui-scripts",
        &gui_scripts,
      ));
    }

    if let Some(entry_points) = context.get("project.entry-points") {
      diagnostics
        .extend(Self::validate_entry_points_table(document, &entry_points));
    }

    diagnostics
  }
}

impl ProjectEntryPointsRule {
  fn is_group_segment(segment: &str) -> bool {
    !segment.is_empty()
      && segment
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
  }

  fn is_identifier(segment: &str) -> bool {
    segment
      .split('.')
      .all(|part| !part.is_empty() && Self::validate_identifier(part))
  }

  fn is_identifier_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
  }

  fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
  }

  fn nested_group_diagnostic(
    document: &Document,
    location: &str,
    node: &Node,
  ) -> Diagnostic {
    Diagnostic::error(
      format!(
        "`{location}` must be a string object reference; entry point groups cannot be nested"
      ),
      node.span(&document.content),
    )
  }

  fn string_value_diagnostic(
    document: &Document,
    location: &str,
    node: &Node,
  ) -> Diagnostic {
    Diagnostic::error(
      format!("`{location}` must be a string object reference"),
      node.span(&document.content),
    )
  }

  fn validate_entry_point_name(
    document: &Document,
    location: &str,
    key: &Key,
  ) -> Option<Diagnostic> {
    let name = key.value();

    if name.trim().is_empty() {
      return Some(Diagnostic::error(
        format!("`{location}` name must not be empty"),
        key.span(&document.content),
      ));
    }

    if name != name.trim() {
      return Some(Diagnostic::error(
        format!("`{location}` name must not start or end with whitespace"),
        key.span(&document.content),
      ));
    }

    if name.starts_with('[') {
      return Some(Diagnostic::error(
        format!("`{location}` name must not start with `[`"),
        key.span(&document.content),
      ));
    }

    if name.contains('=') {
      return Some(Diagnostic::error(
        format!("`{location}` name must not contain `=`"),
        key.span(&document.content),
      ));
    }

    None
  }

  fn validate_entry_point_value(
    document: &Document,
    location: &str,
    node: &Node,
  ) -> Vec<Diagnostic> {
    match node {
      Node::Str(string) => {
        if string.value().trim().is_empty() {
          vec![Diagnostic::error(
            format!("`{location}` must not be empty"),
            node.span(&document.content),
          )]
        } else {
          Self::validate_object_reference(
            location,
            string.value(),
            node.span(&document.content),
          )
        }
      }
      Node::Table(_) => {
        vec![Self::nested_group_diagnostic(document, location, node)]
      }
      _ => vec![Self::string_value_diagnostic(document, location, node)],
    }
  }

  fn validate_entry_points_table(
    document: &Document,
    entry_points: &Node,
  ) -> Vec<Diagnostic> {
    let Some(table) = entry_points.as_table() else {
      return vec![Diagnostic::error(
        "`project.entry-points` must be a table of entry point groups",
        entry_points.span(&document.content),
      )];
    };

    let mut diagnostics = Vec::new();

    for (group_key, group) in table.entries().read().iter() {
      diagnostics.extend(Self::validate_group(
        document,
        group_key.value(),
        group_key,
        group,
      ));
    }

    diagnostics
  }

  fn validate_extras(
    location: &str,
    extras: &str,
    range: lsp::Range,
  ) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if extras.is_empty() {
      diagnostics.push(Diagnostic::error(
        format!("`{location}` extras must not be empty"),
        range,
      ));

      return diagnostics;
    }

    for extra in extras.split(',').map(str::trim) {
      if extra.is_empty() {
        diagnostics.push(Diagnostic::error(
          format!("`{location}` extras must not be empty"),
          range,
        ));

        continue;
      }

      if pep508_rs::ExtraName::from_str(extra).is_err() {
        diagnostics.push(Diagnostic::error(
          format!(
            "`{location}` extra `{extra}` must be a valid PEP 508 extra name"
          ),
          range,
        ));
      }
    }

    diagnostics
  }

  fn validate_group(
    document: &Document,
    name: &str,
    key: &Key,
    node: &Node,
  ) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(Self::validate_group_name(document, name, key));

    match name {
      "console_scripts" => {
        diagnostics.push(Diagnostic::error(
          "`project.entry-points.console_scripts` is not allowed; use `[project.scripts]` instead",
          key.span(&document.content),
        ));
      }
      "gui_scripts" => {
        diagnostics.push(Diagnostic::error(
          "`project.entry-points.gui_scripts` is not allowed; use `[project.gui-scripts]` instead",
          key.span(&document.content),
        ));
      }
      _ => {}
    }

    let Some(table) = node.as_table() else {
      diagnostics.push(Diagnostic::error(
        format!(
          "`project.entry-points.{name}` must be a table of entry points"
        ),
        node.span(&document.content),
      ));

      return diagnostics;
    };

    for (entry_key, entry_value) in table.entries().read().iter() {
      let location =
        format!("project.entry-points.{name}.{}", entry_key.value());

      if let Some(diagnostic) =
        Self::validate_entry_point_name(document, &location, entry_key)
      {
        diagnostics.push(diagnostic);
      }

      diagnostics.extend(Self::validate_entry_point_value(
        document,
        &location,
        entry_value,
      ));
    }

    diagnostics
  }

  fn validate_group_name(
    document: &Document,
    name: &str,
    key: &Key,
  ) -> Option<Diagnostic> {
    if name.split('.').all(Self::is_group_segment) {
      None
    } else {
      Some(Diagnostic::error(
        "`project.entry-points` group names must match `^\\w+(\\.\\w+)*$`",
        key.span(&document.content),
      ))
    }
  }

  fn validate_identifier(value: &str) -> bool {
    let mut chars = value.chars();

    let Some(first) = chars.next() else {
      return false;
    };

    if !Self::is_identifier_start(first) {
      return false;
    }

    chars.all(Self::is_identifier_continue)
  }

  fn validate_object_reference(
    location: &str,
    raw: &str,
    range: lsp::Range,
  ) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let trimmed = raw.trim();

    if trimmed.is_empty() {
      return vec![Diagnostic::error(
        format!("`{location}` must not be empty"),
        range,
      )];
    }

    let (reference, extras) = match trimmed.split_once('[') {
      Some((reference, extras)) => {
        if !extras.trim_end().ends_with(']') {
          diagnostics.push(Diagnostic::error(
            format!("`{location}` extras must be closed with `]`"),
            range,
          ));
        } else if let Some(extra_list) =
          extras.trim_end().strip_suffix(']').map(str::trim)
        {
          diagnostics
            .extend(Self::validate_extras(location, extra_list, range));
        }

        (reference.trim_end(), Some(extras))
      }
      None => (trimmed, None),
    };

    if let Some(diagnostic) =
      Self::validate_reference(location, reference, range)
    {
      diagnostics.push(diagnostic);
    }

    if extras.is_some() {
      diagnostics.push(Diagnostic::warning(
        format!(
          "`{location}` uses extras in entry point definitions; extras are deprecated for entry points and may be ignored by consumers"
        ),
        range,
      ));
    }

    diagnostics
  }

  fn validate_reference(
    location: &str,
    reference: &str,
    range: lsp::Range,
  ) -> Option<Diagnostic> {
    let mut parts = reference.splitn(2, ':').map(str::trim);

    let module = parts.next().unwrap_or_default();
    let qualname = parts.next();

    if !Self::is_identifier(module) {
      return Some(Diagnostic::error(
        format!(
          "`{location}` must reference an importable module path (e.g. `package.module`) optionally followed by `:qualname`"
        ),
        range,
      ));
    }

    if let Some(qualname) = qualname
      && (qualname.is_empty() || !Self::is_identifier(qualname))
    {
      return Some(Diagnostic::error(
        format!(
          "`{location}` object reference after `:` must be a dotted Python identifier"
        ),
        range,
      ));
    }

    None
  }

  fn validate_scripts_table(
    document: &Document,
    field: &str,
    node: &Node,
  ) -> Vec<Diagnostic> {
    let Some(table) = node.as_table() else {
      return vec![Diagnostic::error(
        format!("`{field}` must be a table of entry points"),
        node.span(&document.content),
      )];
    };

    let mut diagnostics = Vec::new();

    for (key, value) in table.entries().read().iter() {
      let location = format!("{field}.{}", key.value());

      if let Some(diagnostic) =
        Self::validate_entry_point_name(document, &location, key)
      {
        diagnostics.push(diagnostic);
      }

      diagnostics
        .extend(Self::validate_entry_point_value(document, &location, value));
    }

    diagnostics
  }
}
