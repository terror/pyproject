use super::*;

pub(crate) struct ProjectClassifiersRule;

impl Rule for ProjectClassifiersRule {
  fn display_name(&self) -> &'static str {
    "Project Classifiers"
  }

  fn id(&self) -> &'static str {
    "project-classifiers"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let Some(project) = context.project() else {
      return Vec::new();
    };

    let Some(classifiers) = project.try_get("classifiers").ok() else {
      return Vec::new();
    };

    let document = context.document();

    let mut diagnostics = Vec::new();

    let Some(array) = classifiers.as_array() else {
      diagnostics.push(lsp::Diagnostic {
        message: "`project.classifiers` must be an array of strings"
          .to_string(),
        range: classifiers.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      });

      return diagnostics;
    };

    let mut seen = HashSet::new();

    for item in array.items().read().iter() {
      match item.as_str() {
        Some(string) => {
          let value = string.value();

          if !seen.insert(value) {
            diagnostics.push(lsp::Diagnostic {
              message: format!(
                "`project.classifiers` contains duplicate classifier `{value}`"
              ),
              range: item.range(&document.content),
              severity: Some(lsp::DiagnosticSeverity::ERROR),
              ..Default::default()
            });

            continue;
          }

          if !Self::classifiers().contains(value) {
            diagnostics.push(lsp::Diagnostic {
              message: format!(
                "`project.classifiers` contains an unknown classifier `{value}`"
              ),
              range: item.range(&document.content),
              severity: Some(lsp::DiagnosticSeverity::ERROR),
              ..Default::default()
            });
          }
        }
        None => diagnostics.push(lsp::Diagnostic {
          message: "`project.classifiers` items must be strings".to_string(),
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        }),
      }
    }

    diagnostics
  }
}

impl ProjectClassifiersRule {
  fn classifiers() -> &'static HashSet<&'static str> {
    static CLASSIFIERS: OnceLock<HashSet<&'static str>> = OnceLock::new();

    CLASSIFIERS.get_or_init(|| {
      include_str!("classifiers.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
    })
  }
}
