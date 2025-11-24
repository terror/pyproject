use super::*;

const ALLOWED_FIELDS: &[&str] = &[
  "authors",
  "classifiers",
  "dependencies",
  "description",
  "entry-points",
  "gui-scripts",
  "keywords",
  "license",
  "maintainers",
  "optional-dependencies",
  "readme",
  "requires-python",
  "scripts",
  "urls",
  "version",
];

pub(crate) struct ProjectDynamicRule;

impl Rule for ProjectDynamicRule {
  fn header(&self) -> &'static str {
    "invalid project.dynamic"
  }

  fn id(&self) -> &'static str {
    "project-dynamic"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let Some(project) = context.project() else {
      return Vec::new();
    };

    let Some(dynamic) = project.try_get("dynamic").ok() else {
      return Vec::new();
    };

    let document = context.document();

    let Some(array) = dynamic.as_array() else {
      return vec![lsp::Diagnostic {
        message: "`project.dynamic` must be an array of strings".to_string(),
        range: dynamic.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }];
    };

    let mut diagnostics = Vec::new();

    let mut seen = HashSet::new();

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        diagnostics.push(lsp::Diagnostic {
          message: "`project.dynamic` items must be strings".to_string(),
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
            "`project.dynamic` contains duplicate field `{value}`"
          ),
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        });

        continue;
      }

      if value == "name" {
        diagnostics.push(lsp::Diagnostic {
          message: "`project.dynamic` must not include `name`".to_string(),
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        });

        continue;
      }

      if !ALLOWED_FIELDS.contains(&value) {
        diagnostics.push(lsp::Diagnostic {
          message: format!(
            "`project.dynamic` contains unsupported field `{value}`"
          ),
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        });

        continue;
      }

      if project.try_get(value).is_ok() {
        diagnostics.push(lsp::Diagnostic {
          message: format!(
            "`project.dynamic` field `{value}` must not also be provided statically"
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
