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
    let Some(dynamic) = context.get("project.dynamic") else {
      return Vec::new();
    };

    let document = context.document();

    let Some(array) = dynamic.as_array() else {
      return vec![Diagnostic::error(
        "`project.dynamic` must be an array of strings",
        dynamic.span(&document.content),
      )];
    };

    let mut diagnostics = Vec::new();

    let mut seen = HashSet::new();

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        diagnostics.push(Diagnostic::error(
          "`project.dynamic` items must be strings",
          item.span(&document.content),
        ));

        continue;
      };

      let value = string.value();

      if !seen.insert(value) {
        diagnostics.push(Diagnostic::error(
          format!("`project.dynamic` contains duplicate field `{value}`"),
          item.span(&document.content),
        ));

        continue;
      }

      if value == "name" {
        diagnostics.push(Diagnostic::error(
          "`project.dynamic` must not include `name`",
          item.span(&document.content),
        ));

        continue;
      }

      if !ALLOWED_FIELDS.contains(&value) {
        diagnostics.push(Diagnostic::error(
          format!("`project.dynamic` contains unsupported field `{value}`"),
          item.span(&document.content),
        ));

        continue;
      }

      if context.get(&format!("project.{value}")).is_some() {
        diagnostics.push(Diagnostic::error(
          format!(
            "`project.dynamic` field `{value}` must not also be provided statically"
          ),
          item.span(&document.content),
        ));
      }
    }

    diagnostics
  }
}
