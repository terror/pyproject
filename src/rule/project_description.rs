use super::*;

define_rule! {
  ProjectDescriptionRule {
    id: "project-description",
    message: "invalid `project.description` value",
    run(context) {
      match context.get("project.description") {
        Some(description) if description.is_str() => Vec::new(),
        Some(description) => vec![Diagnostic::error(
          "`project.description` must be a string",
          description.span(context.content()),
        )],
        None => Vec::new()
      }
    }
  }
}
