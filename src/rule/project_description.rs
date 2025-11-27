use super::*;

define_rule! {
  ProjectDescriptionRule {
    id: "project-description",
    message: "invalid `project.description` value",
    run(context) {
      let Some(description) = context.get("project.description") else {
        return Vec::new();
      };

      let document = context.document();

      if description.is_str() {
        Vec::new()
      } else {
        vec![Diagnostic::error(
          "`project.description` must be a string",
          description.span(&document.content),
        )]
      }
    }
  }
}
