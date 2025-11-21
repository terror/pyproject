use super::*;

pub(crate) struct ProjectNameRule;

impl Rule for ProjectNameRule {
  fn display_name(&self) -> &'static str {
    "Project Name"
  }

  fn id(&self) -> &'static str {
    "project-name"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let document = context.document();

    let tree = context.tree().clone().into_dom();

    let Some(project) = tree.try_get("project").ok() else {
      return Vec::new();
    };

    let name = project.try_get("name").ok();

    let diagnostic = match name {
      Some(name) if !name.is_str() => Some(self.diagnostic(lsp::Diagnostic {
        message: "`project.name` must be a string".to_string(),
        range: name.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })),
      Some(ref name @ Node::Str(ref string)) if string.value().is_empty() => {
        Some(self.diagnostic(lsp::Diagnostic {
          message: "`project.name` must not be empty".to_string(),
          range: name.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        }))
      }
      None => Some(self.diagnostic(lsp::Diagnostic {
        message: "missing required key `project.name`".to_string(),
        range: project.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })),
      _ => None,
    };

    diagnostic
      .map(|diagnostic| vec![diagnostic])
      .unwrap_or_default()
  }
}
