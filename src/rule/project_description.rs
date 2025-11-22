use super::*;

pub(crate) struct ProjectDescriptionRule;

impl Rule for ProjectDescriptionRule {
  fn display_name(&self) -> &'static str {
    "Project Description"
  }

  fn id(&self) -> &'static str {
    "project-description"
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

    let Some(description) = project.try_get("description").ok() else {
      return Vec::new();
    };

    if description.is_str() {
      Vec::new()
    } else {
      vec![self.diagnostic(lsp::Diagnostic {
        message: "`project.description` must be a string".to_string(),
        range: description.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })]
    }
  }
}
