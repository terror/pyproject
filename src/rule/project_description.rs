use super::*;

pub(crate) struct ProjectDescriptionRule;

impl Rule for ProjectDescriptionRule {
  fn header(&self) -> &'static str {
    "invalid project.description"
  }

  fn id(&self) -> &'static str {
    "project-description"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let Some(description) = context.get("project.description") else {
      return Vec::new();
    };

    let document = context.document();

    if description.is_str() {
      Vec::new()
    } else {
      vec![lsp::Diagnostic {
        message: "`project.description` must be a string".to_string(),
        range: description.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }]
    }
  }
}
