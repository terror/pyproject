use super::*;

pub(crate) struct SemanticRule;

impl Rule for SemanticRule {
  fn header(&self) -> &'static str {
    "invalid project structure"
  }

  fn id(&self) -> &'static str {
    "semantic-errors"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let document = context.document();

    let Some(dom) = context.get("") else {
      return Vec::new();
    };

    match dom.validate() {
      Ok(()) => Vec::new(),
      Err(errors) => errors
        .into_iter()
        .flat_map(|error| Self::diagnostics_for_error(document, error))
        .collect(),
    }
  }
}

impl SemanticRule {
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

  fn diagnostics_for_error(
    document: &Document,
    error: SemanticError,
  ) -> Vec<Diagnostic> {
    match error {
      SemanticError::UnexpectedSyntax { syntax } => {
        let kind = format!("{:?}", syntax.kind()).to_lowercase();

        let text = match &syntax {
          SyntaxElement::Node(node) => node.text().to_string(),
          SyntaxElement::Token(token) => token.text().to_string(),
        };

        let text = text.trim();

        vec![Self::diagnostic_for_range(
          document,
          syntax.text_range(),
          format!("unexpected {kind} `{text}`"),
        )]
      }
      SemanticError::InvalidEscapeSequence { string } => {
        vec![Self::diagnostic_for_range(
          document,
          string.text_range(),
          "the string contains invalid escape sequence(s)".to_string(),
        )]
      }
      SemanticError::ConflictingKeys { key, other } => {
        let message =
          format!("conflicting keys: `{key}` conflicts with `{other}`");

        key
          .text_ranges()
          .chain(other.text_ranges())
          .next()
          .map(|range| {
            vec![Self::diagnostic_for_range(document, range, message)]
          })
          .unwrap_or_default()
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
            vec![Self::diagnostic_for_range(document, range, message.clone())]
          })
          .unwrap_or_default()
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
            vec![Self::diagnostic_for_range(document, range, message.clone())]
          })
          .unwrap_or_default()
      }
      SemanticError::Query(query_error) => {
        vec![Diagnostic::new(
          query_error.to_string(),
          lsp::Range {
            start: document.content.byte_to_lsp_position(0),
            end: document.content.byte_to_lsp_position(0),
          },
          lsp::DiagnosticSeverity::ERROR,
        )]
      }
    }
  }
}
