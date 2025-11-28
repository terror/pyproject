use super::*;

define_rule! {
  ProjectDependenciesRule {
    id: "project-dependencies",
    message: "invalid `project.dependencies` configuration",
    run(context) {
      let Some(dependencies) = context.get("project.dependencies") else {
        return Vec::new();
      };

      let mut diagnostics = Vec::new();

      let Some(array) = dependencies.as_array() else {
        diagnostics.push(Diagnostic::error(
          "`project.dependencies` must be an array of PEP 508 strings",
          dependencies.span(context.content()),
        ));

        return diagnostics;
      };

      for item in array.items().read().iter() {
        let Some(string) = item.as_str() else {
          diagnostics.push(Diagnostic::error(
            "`project.dependencies` items must be strings",
            item.span(context.content()),
          ));

          continue;
        };

        let value = string.value();

        match Requirement::<VerbatimUrl>::from_str(value) {
          Ok(requirement) => {
            if let Some(raw_name) =
              RuleContext::extract_dependency_name(value)
            {
              let normalized = requirement.name.to_string();

              if raw_name != normalized {
                diagnostics.push(Diagnostic::error(
                  format!(
                    "`project.dependencies` package name `{raw_name}` must be normalized (use `{normalized}`)"
                  ),
                  item.span(context.content()),
                ));
              }
            }
          }
          Err(error) => diagnostics.push(Diagnostic::error(
            format!(
              "`project.dependencies` item `{value}` is not a valid PEP 508 dependency: {}",
              error.message.to_string().to_lowercase()
            ),
            item.span(context.content()),
          )),
        }
      }

      diagnostics
    }
  }
}
