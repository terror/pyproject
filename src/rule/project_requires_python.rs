use super::*;

define_rule! {
  ProjectRequiresPythonRule {
    id: "project-requires-python",
    message: "invalid `project.requires-python` configuration",
    run(context) {
      let Some(requires_python) = context.get("project.requires-python") else {
        return Vec::new();
      };

      let document = context.document();

      match requires_python.as_str() {
        Some(string) => {
          let value = string.value();

          if value.trim().is_empty() {
            return vec![Diagnostic::error(
              "`project.requires-python` must not be empty",
              requires_python.span(&document.content),
            )];
          }

          match VersionSpecifiers::from_str(value) {
            Ok(_) => Vec::new(),
            Err(error) => vec![Diagnostic::error(
              format!(
                "`project.requires-python` must be a valid PEP 440 version specifier: {error}"
              ),
              requires_python.span(&document.content),
            )],
          }
        }
        None => vec![Diagnostic::error(
          "`project.requires-python` must be a string",
          requires_python.span(&document.content),
        )],
      }
    }
  }
}
