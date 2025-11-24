use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Diagnostic {
  pub(crate) message: String,
  pub(crate) severity: Option<lsp::DiagnosticSeverity>,
}

impl From<Diagnostic> for lsp::Diagnostic {
  fn from(value: Diagnostic) -> Self {
    lsp::Diagnostic {
      message: value.message,
      severity: value.severity,
      ..Default::default()
    }
  }
}

impl Diagnostic {
  pub(crate) fn error(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
      severity: Some(lsp::DiagnosticSeverity::ERROR),
    }
  }
}
