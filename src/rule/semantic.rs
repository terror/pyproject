use super::*;

pub(crate) struct SemanticRule;

impl Rule for SemanticRule {
  fn header(&self) -> &'static str {
    "invalid document structure"
  }

  fn id(&self) -> &'static str {
    "semantic-errors"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let document = context.document();

    match context.tree().clone().into_dom().validate() {
      Ok(()) => Vec::new(),
      Err(errors) => errors
        .into_iter()
        .filter_map(|error| {
          Self::diagnostic_for_error(
            document,
            error,
            !context.tree().errors.is_empty(),
          )
        })
        .collect(),
    }
  }
}

impl SemanticRule {
  fn diagnostic_for_error(
    document: &Document,
    error: SemanticError,
    has_syntax_errors: bool,
  ) -> Option<Diagnostic> {
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
          syntax.text_range().span(&document.content),
        ))
      }
      SemanticError::InvalidEscapeSequence { string } if !has_syntax_errors => {
        Some(Diagnostic::error(
          "the string contains invalid escape sequence(s)".to_string(),
          string.text_range().span(&document.content),
        ))
      }
      SemanticError::ConflictingKeys { key, other } => {
        let message =
          format!("conflicting keys: `{key}` conflicts with `{other}`");

        key
          .text_ranges()
          .chain(other.text_ranges())
          .next()
          .map(|range| {
            Diagnostic::error(message, range.span(&document.content))
          })
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
          .map(|range| {
            Diagnostic::error(message, range.span(&document.content))
          })
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
          .map(|range| {
            Diagnostic::error(message, range.span(&document.content))
          })
      }
      SemanticError::Query(query_error) => Some(Diagnostic::error(
        query_error.to_string(),
        (0, 0).span(&document.content),
      )),
      SemanticError::UnexpectedSyntax { .. }
      | SemanticError::InvalidEscapeSequence { .. } => None,
    }
  }
}
