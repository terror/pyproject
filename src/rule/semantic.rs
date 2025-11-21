use super::*;

pub(crate) struct SemanticRule;

impl Rule for SemanticRule {
  fn display_name(&self) -> &'static str {
    "Semantic Errors"
  }

  fn id(&self) -> &'static str {
    "semantic-errors"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let document = context.document();

    match context.tree().clone().into_dom().validate() {
      Ok(()) => Vec::new(),
      Err(errors) => errors
        .into_iter()
        .flat_map(|error| self.diagnostics_for_error(document, error))
        .collect(),
    }
  }
}

impl SemanticRule {
  fn diagnostic_for_range(
    &self,
    document: &Document,
    range: TextRange,
    message: String,
  ) -> lsp::Diagnostic {
    self.diagnostic(lsp::Diagnostic {
      message,
      range: lsp::Range {
        start: document.content.byte_to_lsp_position(range.start().into()),
        end: document.content.byte_to_lsp_position(range.end().into()),
      },
      severity: Some(lsp::DiagnosticSeverity::ERROR),
      ..Default::default()
    })
  }

  fn diagnostics_for_error(
    &self,
    document: &Document,
    error: SemanticError,
  ) -> Vec<lsp::Diagnostic> {
    match error {
      SemanticError::UnexpectedSyntax { syntax } => {
        let kind = format!("{:?}", syntax.kind()).to_lowercase();

        let text = match &syntax {
          SyntaxElement::Node(node) => node.text().to_string(),
          SyntaxElement::Token(token) => token.text().to_string(),
        };

        let text = text.trim();

        vec![self.diagnostic_for_range(
          document,
          syntax.text_range(),
          format!("unexpected {kind} `{text}`"),
        )]
      }
      SemanticError::InvalidEscapeSequence { string } => {
        vec![self.diagnostic_for_range(
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
            vec![self.diagnostic_for_range(document, range, message)]
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
            vec![self.diagnostic_for_range(document, range, message.clone())]
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
            vec![self.diagnostic_for_range(document, range, message.clone())]
          })
          .unwrap_or_default()
      }
      SemanticError::Query(query_error) => {
        vec![self.diagnostic(lsp::Diagnostic {
          range: lsp::Range {
            start: document.content.byte_to_lsp_position(0),
            end: document.content.byte_to_lsp_position(0),
          },
          message: query_error.to_string(),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        })]
      }
    }
  }
}
