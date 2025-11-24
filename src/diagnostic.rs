use super::*;

#[derive(Debug)]
pub(crate) struct Diagnostic {
  /// A short header summarizing the diagnostic.
  pub(crate) header: String,
  /// A unique identifier for the diagnostic.
  pub(crate) id: String,
  /// A detailed message describing the diagnostic.
  pub(crate) message: String,
  /// The range in the source code where the diagnostic applies.
  pub(crate) range: lsp::Range,
  /// The severity level of the diagnostic.
  pub(crate) severity: lsp::DiagnosticSeverity,
}

impl Into<lsp::Diagnostic> for Diagnostic {
  fn into(self) -> lsp::Diagnostic {
    lsp::Diagnostic {
      code: Some(lsp::NumberOrString::String(self.id)),
      message: self.message,
      range: self.range,
      severity: Some(self.severity),
      source: Some("pyproject".to_string()),
      ..Default::default()
    }
  }
}
