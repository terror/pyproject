use super::*;

pub(crate) struct ProjectPeopleRule;

impl Rule for ProjectPeopleRule {
  fn header(&self) -> &'static str {
    "invalid project authors or maintainers"
  }

  fn id(&self) -> &'static str {
    "project-people"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let Some(project) = context.project() else {
      return Vec::new();
    };

    let document = context.document();

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
  ) -> Diagnostic {
    Diagnostic::new(
      format!("`{field}` must be an array of inline tables"),
      node.range(&document.content),
      lsp::DiagnosticSeverity::ERROR,
    )
  }

  fn invalid_item_kind(
    document: &Document,
    field: &str,
    node: &Node,
  ) -> Diagnostic {
    Diagnostic::new(
      format!("`{field}` items must use inline tables"),
      node.range(&document.content),
      lsp::DiagnosticSeverity::ERROR,
    )
  }

  fn invalid_item_type(
    document: &Document,
    field: &str,
    node: &Node,
  ) -> Diagnostic {
    Diagnostic::new(
      format!("`{field}` items must be inline tables"),
      node.range(&document.content),
      lsp::DiagnosticSeverity::ERROR,
    )
  }

  fn invalid_key(document: &Document, field: &str, key: &Key) -> Diagnostic {
    Diagnostic::new(
      format!("`{field}` items may only contain `name` or `email`"),
      key.range(&document.content),
      lsp::DiagnosticSeverity::ERROR,
    )
  }

  fn validate_email(
    document: &Document,
    field: &str,
    node: &Node,
  ) -> Vec<Diagnostic> {
    match node {
      Node::Str(string) => {
        let value = string.value();

        match Self::validate_email_value(value) {
          Ok(()) => Vec::new(),
          Err(_) => vec![Diagnostic::new(
            format!("`{field}.email` must be a valid email address"),
            node.range(&document.content),
            lsp::DiagnosticSeverity::ERROR,
          )],
        }
      }
      _ => vec![Diagnostic::new(
        format!("`{field}.email` must be a string"),
        node.range(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      )],
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
  ) -> Vec<Diagnostic> {
    match node {
      Node::Str(string) => {
        let value = string.value();

        match Self::validate_name_value(value) {
          Ok(()) => Vec::new(),
          Err(_) => vec![Diagnostic::new(
            format!("`{field}.name` must be a valid email name without commas"),
            node.range(&document.content),
            lsp::DiagnosticSeverity::ERROR,
          )],
        }
      }
      _ => vec![Diagnostic::new(
        format!("`{field}.name` must be a string"),
        node.range(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      )],
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
  ) -> Vec<Diagnostic> {
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
  ) -> Vec<Diagnostic> {
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

#[cfg(test)]
mod tests {
  use {super::*, pretty_assertions::assert_eq};

  #[test]
  fn validate_email_value_valid_simple() {
    assert_eq!(
      ProjectPeopleRule::validate_email_value("user@example.com"),
      Ok(())
    );
  }

  #[test]
  fn validate_email_value_valid_with_subdomain() {
    assert_eq!(
      ProjectPeopleRule::validate_email_value("user@mail.example.com"),
      Ok(())
    );
  }

  #[test]
  fn validate_email_value_valid_with_plus() {
    assert_eq!(
      ProjectPeopleRule::validate_email_value("user+tag@example.com"),
      Ok(())
    );
  }

  #[test]
  fn validate_email_value_valid_with_dots() {
    assert_eq!(
      ProjectPeopleRule::validate_email_value("first.last@example.com"),
      Ok(())
    );
  }

  #[test]
  fn validate_email_value_valid_with_numbers() {
    assert_eq!(
      ProjectPeopleRule::validate_email_value("user123@example.com"),
      Ok(())
    );
  }

  #[test]
  fn validate_email_value_valid_with_hyphens() {
    assert_eq!(
      ProjectPeopleRule::validate_email_value("user-name@example.com"),
      Ok(())
    );
  }

  #[test]
  fn validate_email_value_empty_string() {
    assert!(ProjectPeopleRule::validate_email_value("").is_err());
  }

  #[test]
  fn validate_email_value_whitespace_only() {
    assert!(ProjectPeopleRule::validate_email_value("   ").is_err());
  }

  #[test]
  fn validate_email_value_with_display_name() {
    assert_eq!(
      ProjectPeopleRule::validate_email_value("John Doe <user@example.com>"),
      Err("email must not include a display name".to_string())
    );
  }

  #[test]
  fn validate_email_value_with_display_name_quoted() {
    assert_eq!(
      ProjectPeopleRule::validate_email_value(
        "\"John Doe\" <user@example.com>"
      ),
      Err("email must not include a display name".to_string())
    );
  }

  #[test]
  fn validate_email_value_multiple_addresses() {
    assert_eq!(
      ProjectPeopleRule::validate_email_value(
        "user1@example.com, user2@example.com"
      ),
      Err("email must contain exactly one address".to_string())
    );
  }

  #[test]
  fn validate_email_value_no_at_sign() {
    assert!(ProjectPeopleRule::validate_email_value("notanemail").is_err());
  }

  #[test]
  fn validate_email_value_with_surrounding_whitespace() {
    assert_eq!(
      ProjectPeopleRule::validate_email_value("  user@example.com  "),
      Ok(())
    );
  }

  #[test]
  fn validate_name_value_valid_simple() {
    assert_eq!(ProjectPeopleRule::validate_name_value("John Doe"), Ok(()));
  }

  #[test]
  fn validate_name_value_valid_single_word() {
    assert_eq!(ProjectPeopleRule::validate_name_value("Alice"), Ok(()));
  }

  #[test]
  fn validate_name_value_valid_with_numbers() {
    assert_eq!(ProjectPeopleRule::validate_name_value("Alice 123"), Ok(()));
  }

  #[test]
  fn validate_name_value_valid_with_special_chars() {
    assert_eq!(
      ProjectPeopleRule::validate_name_value("Jean-Paul O'Brien"),
      Ok(())
    );
  }

  #[test]
  fn validate_name_value_valid_with_unicode() {
    assert_eq!(
      ProjectPeopleRule::validate_name_value("José García"),
      Ok(())
    );
  }

  #[test]
  fn validate_name_value_valid_with_dots() {
    assert_eq!(
      ProjectPeopleRule::validate_name_value("Dr. John Doe"),
      Ok(())
    );
  }

  #[test]
  fn validate_name_value_valid_multiple_words() {
    assert_eq!(
      ProjectPeopleRule::validate_name_value("Mary Jane Watson"),
      Ok(())
    );
  }

  #[test]
  fn validate_name_value_empty_string() {
    assert_eq!(
      ProjectPeopleRule::validate_name_value(""),
      Err("name must not be empty".to_string())
    );
  }

  #[test]
  fn validate_name_value_whitespace_only() {
    assert_eq!(
      ProjectPeopleRule::validate_name_value("   "),
      Err("name must not be empty".to_string())
    );
  }

  #[test]
  fn validate_name_value_with_comma() {
    assert_eq!(
      ProjectPeopleRule::validate_name_value("Doe, John"),
      Err("name must not contain commas".to_string())
    );
  }

  #[test]
  fn validate_name_value_with_comma_in_middle() {
    assert_eq!(
      ProjectPeopleRule::validate_name_value("John, Doe"),
      Err("name must not contain commas".to_string())
    );
  }

  #[test]
  fn validate_name_value_with_surrounding_whitespace() {
    assert_eq!(
      ProjectPeopleRule::validate_name_value("  John Doe  "),
      Ok(())
    );
  }

  #[test]
  fn validate_name_value_with_parentheses() {
    assert_eq!(
      ProjectPeopleRule::validate_name_value("John Doe (Jr)"),
      Ok(())
    );
  }

  #[test]
  fn validate_name_value_with_brackets() {
    assert_eq!(
      ProjectPeopleRule::validate_name_value("John Doe [Maintainer]"),
      Ok(())
    );
  }
}
