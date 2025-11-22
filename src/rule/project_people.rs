use super::*;

pub(crate) struct ProjectPeopleRule;

impl Rule for ProjectPeopleRule {
  fn display_name(&self) -> &'static str {
    "Project Authors and Maintainers"
  }

  fn id(&self) -> &'static str {
    "project-people"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let document = context.document();

    let tree = context.tree().clone().into_dom();

    let Ok(project) = tree.try_get("project") else {
      return Vec::new();
    };

    let mut diagnostics = Vec::new();

    if let Ok(authors) = project.try_get("authors") {
      diagnostics.extend(Self::validate_people_field(
        document,
        "project.authors",
        authors,
      ));
    }

    if let Ok(maintainers) = project.try_get("maintainers") {
      diagnostics.extend(Self::validate_people_field(
        document,
        "project.maintainers",
        maintainers,
      ));
    }

    diagnostics
  }
}

impl ProjectPeopleRule {
  const PLACEHOLDER_EMAIL: &'static str = "example@example.com";

  fn invalid_field_type(
    document: &Document,
    field: &str,
    node: &Node,
  ) -> lsp::Diagnostic {
    lsp::Diagnostic {
      message: format!("`{field}` must be an array of inline tables"),
      range: node.range(&document.content),
      severity: Some(lsp::DiagnosticSeverity::ERROR),
      ..Default::default()
    }
  }

  fn invalid_item_kind(
    document: &Document,
    field: &str,
    node: &Node,
  ) -> lsp::Diagnostic {
    lsp::Diagnostic {
      message: format!("`{field}` items must use inline tables"),
      range: node.range(&document.content),
      severity: Some(lsp::DiagnosticSeverity::ERROR),
      ..Default::default()
    }
  }

  fn invalid_item_type(
    document: &Document,
    field: &str,
    node: &Node,
  ) -> lsp::Diagnostic {
    lsp::Diagnostic {
      message: format!("`{field}` items must be inline tables"),
      range: node.range(&document.content),
      severity: Some(lsp::DiagnosticSeverity::ERROR),
      ..Default::default()
    }
  }

  fn invalid_key(
    document: &Document,
    field: &str,
    key: &Key,
  ) -> lsp::Diagnostic {
    lsp::Diagnostic {
      message: format!("`{field}` items may only contain `name` or `email`"),
      range: key.range(&document.content),
      severity: Some(lsp::DiagnosticSeverity::ERROR),
      ..Default::default()
    }
  }

  fn validate_email(
    document: &Document,
    field: &str,
    node: &Node,
  ) -> Vec<lsp::Diagnostic> {
    match node {
      Node::Str(string) => {
        let value = string.value();

        match Self::validate_email_value(value) {
          Ok(()) => Vec::new(),
          Err(_) => vec![lsp::Diagnostic {
            message: format!("`{field}.email` must be a valid email address"),
            range: node.range(&document.content),
            severity: Some(lsp::DiagnosticSeverity::ERROR),
            ..Default::default()
          }],
        }
      }
      _ => vec![lsp::Diagnostic {
        message: format!("`{field}.email` must be a string"),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }],
    }
  }

  fn validate_email_value(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
      return Err("email must not be empty".into());
    }

    let addresses = addrparse(value).map_err(|error| error.to_string())?;

    match addresses.as_slice() {
      [MailAddr::Single(single)]
        if single.display_name.is_none() && !single.addr.trim().is_empty() =>
      {
        Ok(())
      }
      [MailAddr::Single(_)] => {
        Err("email must not include a display name".into())
      }
      _ => Err("email must contain exactly one address".into()),
    }
  }

  fn validate_name(
    document: &Document,
    field: &str,
    node: &Node,
  ) -> Vec<lsp::Diagnostic> {
    match node {
      Node::Str(string) => {
        let value = string.value();

        match Self::validate_name_value(value) {
          Ok(()) => Vec::new(),
          Err(_) => vec![lsp::Diagnostic {
            message: format!(
              "`{field}.name` must be a valid email name without commas"
            ),
            range: node.range(&document.content),
            severity: Some(lsp::DiagnosticSeverity::ERROR),
            ..Default::default()
          }],
        }
      }
      _ => vec![lsp::Diagnostic {
        message: format!("`{field}.name` must be a string"),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }],
    }
  }

  fn validate_name_value(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
      return Err("name must not be empty".into());
    }

    if value.contains(',') {
      return Err("name must not contain commas".into());
    }

    let display = format!("{value} <{}>", Self::PLACEHOLDER_EMAIL);

    let addresses = addrparse(&display).map_err(|error| error.to_string())?;

    match addresses.as_slice() {
      [MailAddr::Single(single)] if single.display_name.is_some() => Ok(()),
      _ => Err("name must parse as a single address".into()),
    }
  }

  fn validate_people_field(
    document: &Document,
    field: &'static str,
    node: Node,
  ) -> Vec<lsp::Diagnostic> {
    let Some(array) = node.as_array() else {
      return vec![Self::invalid_field_type(document, field, &node)];
    };

    let mut diagnostics = Vec::new();

    for item in array.items().read().iter() {
      diagnostics.extend(Self::validate_person(document, field, item));
    }

    diagnostics
  }

  fn validate_person(
    document: &Document,
    field: &str,
    node: &Node,
  ) -> Vec<lsp::Diagnostic> {
    let Some(table) = node.as_table() else {
      return vec![Self::invalid_item_type(document, field, node)];
    };

    let mut diagnostics = Vec::new();

    if table.kind() != TableKind::Inline {
      diagnostics.push(Self::invalid_item_kind(document, field, node));
    }

    for (key, value) in table.entries().read().iter() {
      match key.value() {
        "email" => {
          diagnostics.extend(Self::validate_email(document, field, value));
        }
        "name" => {
          diagnostics.extend(Self::validate_name(document, field, value));
        }
        _ => diagnostics.push(Self::invalid_key(document, field, key)),
      }
    }

    diagnostics
  }
}
