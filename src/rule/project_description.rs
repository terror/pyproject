use super::*;

pub(crate) struct ProjectDescriptionRule;

impl Rule for ProjectDescriptionRule {
  fn display(&self) -> &'static str {
    "invalid `project.description` value"
  }

  fn id(&self) -> &'static str {
    "project-description"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(description) = context.get("project.description") else {
      return Vec::new();
    };

    let document = context.document();

    if description.is_str() {
      Vec::new()
    } else {
      vec![Diagnostic::error(
        "`project.description` must be a string",
        description.span(&document.content),
      )]
    }
  }
}
