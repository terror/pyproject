use super::*;

#[derive(Debug)]
pub(crate) struct Diagnostic {
  /// A short header summarizing the diagnostic.
  pub(crate) display: String,
  /// A unique identifier for the diagnostic.
  pub(crate) id: String,
  /// A detailed message describing the diagnostic.
  pub(crate) message: String,
  /// An optional edit that resolves the diagnostic.
  pub(crate) quickfix: Option<Quickfix>,
  /// The range in the source code where the diagnostic applies.
  pub(crate) range: lsp::Range,
  /// The severity level of the diagnostic.
  pub(crate) severity: lsp::DiagnosticSeverity,
}

impl Diagnostic {
  pub(crate) fn error(message: impl Into<String>, range: lsp::Range) -> Self {
    Self::new(message, range, lsp::DiagnosticSeverity::ERROR)
  }

  pub(crate) fn new(
    message: impl Into<String>,
    range: lsp::Range,
    severity: lsp::DiagnosticSeverity,
  ) -> Self {
    Self {
      display: String::new(),
      id: String::new(),
      message: message.into(),
      quickfix: None,
      range,
      severity,
    }
  }

  pub(crate) fn quickfix(self, quickfix: Quickfix) -> Self {
    Self {
      quickfix: Some(quickfix),
      ..self
    }
  }

  pub(crate) fn warning(message: impl Into<String>, range: lsp::Range) -> Self {
    Self::new(message, range, lsp::DiagnosticSeverity::WARNING)
  }
}

impl From<Diagnostic> for lsp::Diagnostic {
  fn from(value: Diagnostic) -> lsp::Diagnostic {
    (&value).into()
  }
}

impl From<&Diagnostic> for lsp::Diagnostic {
  fn from(value: &Diagnostic) -> lsp::Diagnostic {
    lsp::Diagnostic {
      code: Some(lsp::NumberOrString::String(value.id.clone())),
      message: value.message.clone(),
      range: value.range,
      severity: Some(value.severity),
      source: Some("pyproject".to_string()),
      ..Default::default()
    }
  }
}
