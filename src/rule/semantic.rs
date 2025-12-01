use super::*;

define_rule! {
  /// Reports TOML semantic errors such as conflicting keys or invalid escape sequences.
  SemanticRule {
    id: "semantic-errors",
    message: "invalid document structure",
    run(context) {
      match context.tree().clone().into_dom().validate() {
        Ok(()) => Vec::new(),
        Err(errors) => errors
          .into_iter()
          .filter_map(|error| Self::diagnostic(context, error))
          .collect(),
      }
    }
  }
}

impl SemanticRule {
  fn diagnostic(
    context: &RuleContext<'_>,
    error: SemanticError,
  ) -> Option<Diagnostic> {
    let content = context.content();

    let has_syntax_errors = !context.tree().errors.is_empty();

    match error {
      SemanticError::UnexpectedSyntax { syntax } if !has_syntax_errors => {
        let kind = format!("{:?}", syntax.kind()).to_lowercase();

        let text = match &syntax {
          SyntaxElement::Node(node) => node.text().to_string(),
          SyntaxElement::Token(token) => token.text().to_string(),
        };

        let text = text.trim();

        Some(Diagnostic::error(
          format!("unexpected {kind} `{text}`"),
          syntax.text_range().span(content),
        ))
      }
      SemanticError::InvalidEscapeSequence { string } if !has_syntax_errors => {
        Some(Diagnostic::error(
          "the string contains invalid escape sequence(s)".to_string(),
          string.text_range().span(content),
        ))
      }
      SemanticError::ConflictingKeys { key, other } => {
        let message =
          format!("conflicting keys: `{key}` conflicts with `{other}`");

        key
          .text_ranges()
          .chain(other.text_ranges())
          .next()
          .map(|range| Diagnostic::error(message, range.span(content)))
      }
      SemanticError::ExpectedTable {
        not_table,
        required_by,
      } => {
        let message =
          format!("expected table `{not_table}` required by `{required_by}`");

        not_table
          .text_ranges()
          .chain(required_by.text_ranges())
          .next()
          .map(|range| Diagnostic::error(message, range.span(content)))
      }
      SemanticError::ExpectedArrayOfTables {
        not_array_of_tables,
        required_by,
      } => {
        let message = format!(
          "expected array of tables `{not_array_of_tables}` required by `{required_by}`"
        );

        not_array_of_tables
          .text_ranges()
          .chain(required_by.text_ranges())
          .next()
          .map(|range| Diagnostic::error(message, range.span(content)))
      }
      SemanticError::Query(query_error) => Some(Diagnostic::error(
        query_error.to_string(),
        (0, 0).span(content),
      )),
      SemanticError::UnexpectedSyntax { .. }
      | SemanticError::InvalidEscapeSequence { .. } => None,
    }
  }
}
