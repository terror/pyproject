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
    todo!()
  }
}
