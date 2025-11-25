use super::*;

pub(crate) struct SyntaxRule;

impl Rule for SyntaxRule {
  fn header(&self) -> &'static str {
    "syntax error"
  }

  fn id(&self) -> &'static str {
    "syntax-errors"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let document = context.document();

    context
      .tree()
      .errors
      .clone()
      .into_iter()
      .map(|error| {
        Diagnostic::error(
          error.message,
          lsp::Range {
            start: document
              .content
              .byte_to_lsp_position(error.range.start().into()),
            end: document
              .content
              .byte_to_lsp_position(error.range.end().into()),
          },
        )
      })
      .collect()
  }
}
