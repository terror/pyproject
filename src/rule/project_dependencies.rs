use super::*;

define_rule! {
  /// Validates `project.dependencies` entries as PEP 508 dependency specifiers.
  ///
  /// Ensures all entries are valid PEP 508 strings and that package names
  /// are normalized according to PEP 503.
  ProjectDependenciesRule {
    id: "project-dependencies",
    message: "invalid `project.dependencies` configuration",
    run(context) {
      let mut diagnostics = Vec::new();

      for dependency in context.project_dependencies() {
        match dependency {
          Ok(dependency) => {
            let raw_name = dependency.raw_name();

            let normalized = dependency.requirement.name.as_ref();

            if raw_name != normalized {
              diagnostics.push(Diagnostic::error(
                format!(
                  "`project.dependencies` package name `{raw_name}` must be normalized (use `{normalized}`)"
                ),
                dependency.range,
              ));
            }
          }
          Err(error) => diagnostics.push(match error {
            DependencyError::InvalidRequirement { error, range } => {
              Diagnostic::error(
                format!(
                  "`project.dependencies` item `{}` is not a valid PEP 508 dependency: {}",
                  error.input,
                  error.message.to_string().to_lowercase()
                ),
                range,
              )
            }
            DependencyError::NotArray(range) => Diagnostic::error(
              "`project.dependencies` must be an array of PEP 508 strings",
              range,
            ),
            DependencyError::NotString(range) => Diagnostic::error(
              "`project.dependencies` items must be strings",
              range,
            ),
          }),
        }
      }

      diagnostics
    }
  }
}
