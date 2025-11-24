use super::*;

pub(crate) struct ProjectKeywordsRule;

impl Rule for ProjectKeywordsRule {
  fn header(&self) -> &'static str {
    "invalid project.keywords"
  }

  fn id(&self) -> &'static str {
    "project-keywords"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let Some(project) = context.project() else {
      return Vec::new();
    };

    let Some(keywords) = project.try_get("keywords").ok() else {
      return Vec::new();
    };

    let document = context.document();

    let mut diagnostics = Vec::new();

    let Some(array) = keywords.as_array() else {
      diagnostics.push(Diagnostic::new(
        "`project.keywords` must be an array of strings",
        keywords.range(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      ));

      return diagnostics;
    };

    let mut seen = HashSet::new();

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        diagnostics.push(Diagnostic::new(
          "`project.keywords` items must be strings",
          item.range(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        ));

        continue;
      };

      let value = string.value();

      if !seen.insert(value) {
        diagnostics.push(Diagnostic::new(
          format!("`project.keywords` contains duplicate keyword `{value}`"),
          item.range(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        ));
      }
    }

    diagnostics
  }
}
