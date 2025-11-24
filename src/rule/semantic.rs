use super::*;

pub(crate) struct SemanticRule;

impl Rule for SemanticRule {
  fn header(&self) -> &'static str {
    "conflicting or invalid TOML structure"
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

        Some(Self::diagnostic_for_range(
          document,
          syntax.text_range(),
          format!("unexpected {kind} `{text}`"),
        ))
      }
      SemanticError::InvalidEscapeSequence { string } if !has_syntax_errors => {
        Some(Self::diagnostic_for_range(
          document,
          string.text_range(),
          "the string contains invalid escape sequence(s)".to_string(),
        ))
      }
      SemanticError::ConflictingKeys { key, other } => {
        let message =
          format!("conflicting keys: `{key}` conflicts with `{other}`");

        key
          .text_ranges()
          .chain(other.text_ranges())
          .next()
          .map(|range| Self::diagnostic_for_range(document, range, message))
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
            Self::diagnostic_for_range(document, range, message.clone())
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
            Self::diagnostic_for_range(document, range, message.clone())
          })
      }
      SemanticError::Query(query_error) => Some(Diagnostic::new(
        query_error.to_string(),
        lsp::Range {
          start: document.content.byte_to_lsp_position(0),
          end: document.content.byte_to_lsp_position(0),
        },
        lsp::DiagnosticSeverity::ERROR,
      )),
      SemanticError::UnexpectedSyntax { .. }
      | SemanticError::InvalidEscapeSequence { .. } => None,
    }
  }

  fn diagnostic_for_range(
    document: &Document,
    range: TextRange,
    message: String,
  ) -> Diagnostic {
    Diagnostic::new(
      message,
      lsp::Range {
        start: document.content.byte_to_lsp_position(range.start().into()),
        end: document.content.byte_to_lsp_position(range.end().into()),
      },
      lsp::DiagnosticSeverity::ERROR,
    )
  }
}
