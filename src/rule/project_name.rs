use super::*;

define_rule! {
  /// Validates `project.name` is present and PEP 503 normalized.
  ///
  /// Ensures the project name exists, is a non-empty string, and uses
  /// lowercase with hyphens as separators (no underscores, dots, or mixed case).
  ProjectNameRule {
    id: "project-name",
    message: "invalid value for `project.name`",
    run(context) {
      let Some(project) = context.get("project") else {
        return Vec::new();
      };

      let content = context.content();

      let diagnostic = match context.get("project.name") {
        Some(name) if !name.is_str() => Some(Diagnostic::error(
          "`project.name` must be a string",
          name.span(content),
        )),
        Some(ref name @ Node::Str(ref string)) => {
          let value = string.value();

          if value.is_empty() {
            Some(Diagnostic::error(
              "`project.name` must not be empty",
              name.span(content),
            ))
          } else {
              let normalized = PROJECT_NAME
                .replace_all(value, "-")
                .to_ascii_lowercase();

            if normalized == value {
              None
            } else {
              Some(Diagnostic::error(
                format!(
                  "`project.name` must be PEP 503 normalized (use `{normalized}`)"
                ),
                name.span(content),
              ))
            }
          }
        }
        None => Some(Diagnostic::error(
          "missing required key `project.name`",
          project.span(content),
        )),
        _ => None,
      };

      diagnostic
        .map(|diagnostic| vec![diagnostic])
        .unwrap_or_default()
    }
  }
}
