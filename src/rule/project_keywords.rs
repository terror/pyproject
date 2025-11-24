use super::*;

pub(crate) struct ProjectKeywordsRule;

impl Rule for ProjectKeywordsRule {
  fn header(&self) -> &'static str {
    "project.keywords validation issues"
  }

  fn id(&self) -> &'static str {
    "project-keywords"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(keywords) = context.get("project.keywords") else {
      return Vec::new();
    };

    let document = context.document();

    let mut diagnostics = Vec::new();

    let Some(array) = keywords.as_array() else {
      diagnostics.push(Diagnostic::new(
        "`project.keywords` must be an array of strings",
        keywords.span(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      ));

      return diagnostics;
    };

    let mut seen = HashSet::new();

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        diagnostics.push(Diagnostic::new(
          "`project.keywords` items must be strings",
          item.span(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        ));

        continue;
      };

      let value = string.value();

      if !seen.insert(value) {
        diagnostics.push(Diagnostic::new(
          format!("`project.keywords` contains duplicate keyword `{value}`"),
          item.span(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        ));
      }
    }

    diagnostics
  }
}
