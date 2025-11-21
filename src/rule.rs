use super::*;

pub(crate) use syntax::SyntaxRule;

mod syntax;

pub(crate) trait Rule: Sync {
  /// Helper to annotate diagnostics with rule information.
  fn diagnostic(&self, diagnostic: lsp::Diagnostic) -> lsp::Diagnostic {
    lsp::Diagnostic {
      code: Some(lsp::NumberOrString::String(self.id().to_string())),
      source: Some(format!("just-lsp ({})", self.display_name())),
      ..diagnostic
    }
  }

  /// Human-readable name for the rule.
  fn display_name(&self) -> &'static str;

  /// Unique identifier for the rule.
  fn id(&self) -> &'static str;

  /// Execute the rule and return diagnostics.
  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic>;
}
