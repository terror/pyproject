use super::*;

pub(crate) struct ProjectKeywordsRule;

impl Rule for ProjectKeywordsRule {
  fn display_name(&self) -> &'static str {
    "Project Keywords"
  }

  fn id(&self) -> &'static str {
    "project-keywords"
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

    let Some(keywords) = project.try_get("keywords").ok() else {
      return Vec::new();
    };

    let mut diagnostics = Vec::new();

    let Some(array) = keywords.as_array() else {
      diagnostics.push(lsp::Diagnostic {
        message: "`project.keywords` must be an array of strings".to_string(),
        range: keywords.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      });

      return diagnostics;
    };

    let mut seen = HashSet::new();

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        diagnostics.push(lsp::Diagnostic {
          message: "`project.keywords` items must be strings".to_string(),
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        });

        continue;
      };

      let value = string.value();

      if !seen.insert(value) {
        diagnostics.push(lsp::Diagnostic {
          message: format!(
            "`project.keywords` contains duplicate keyword `{value}`"
          ),
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        });
      }
    }

    diagnostics
  }
}
