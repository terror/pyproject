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
    "project.dynamic values are invalid"
  }

  fn id(&self) -> &'static str {
    "project-dynamic"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
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
      return vec![Diagnostic::new(
        "`project.dynamic` must be an array of strings",
        dynamic.span(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      )];
    };

    let mut diagnostics = Vec::new();

    let mut seen = HashSet::new();

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        diagnostics.push(Diagnostic::new(
          "`project.dynamic` items must be strings",
          item.span(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        ));

        continue;
      };

      let value = string.value();

      if !seen.insert(value) {
        diagnostics.push(Diagnostic::new(
          format!("`project.dynamic` contains duplicate field `{value}`"),
          item.span(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        ));

        continue;
      }

      if value == "name" {
        diagnostics.push(Diagnostic::new(
          "`project.dynamic` must not include `name`",
          item.span(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        ));

        continue;
      }

      if !ALLOWED_FIELDS.contains(&value) {
        diagnostics.push(Diagnostic::new(
          format!("`project.dynamic` contains unsupported field `{value}`"),
          item.span(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        ));

        continue;
      }

      if project.try_get(value).is_ok() {
        diagnostics.push(Diagnostic::new(
          format!(
            "`project.dynamic` field `{value}` must not also be provided statically"
          ),
          item.span(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        ));
      }
    }

    diagnostics
  }
}
