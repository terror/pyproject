use super::*;

pub(crate) struct SyntaxRule;

impl Rule for SyntaxRule {
  fn display_name(&self) -> &'static str {
    "Syntax Errors"
  }

  fn id(&self) -> &'static str {
    "syntax-errors"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    let document = context.document();

    context
      .tree()
      .errors
      .clone()
      .into_iter()
      .map(|error| {
        self.diagnostic(lsp::Diagnostic {
          range: lsp::Range {
            start: document
              .content
              .byte_to_lsp_position(error.range.start().into()),
            end: document
              .content
              .byte_to_lsp_position(error.range.end().into()),
          },
          message: error.message,
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        })
      })
      .collect()
  }
}
