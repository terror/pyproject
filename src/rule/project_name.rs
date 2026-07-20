use super::*;

define_rule! {
  /// Validates `project.name` is present and a valid distribution name.
  ///
  /// Ensures the project name exists, is a non-empty string, and follows the
  /// distribution name grammar.
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
          } else if PROJECT_NAME.is_match(value) {
            None
          } else {
            Some(Diagnostic::error(
              "`project.name` must be a valid distribution name",
              name.span(content),
            ))
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
