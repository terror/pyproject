use super::*;

define_rule! {
  /// Validates `project.optional-dependencies` configuration.
  ///
  /// Ensures extra names are valid PEP 508 identifiers, dependency arrays
  /// contain valid PEP 508 strings, and package names are normalized.
  ProjectOptionalDependenciesRule {
    id: "project-optional-dependencies",
    message: "invalid `project.optional-dependencies` configuration",
    run(context) {
      let Some(optional_dependencies) =
        context.get("project.optional-dependencies")
      else {
        return Vec::new();
      };

      let content = context.content();

      let Some(table) = optional_dependencies.as_table() else {
        return vec![Diagnostic::error(
          "`project.optional-dependencies` must be a table",
          optional_dependencies.span(content),
        )];
      };

      let mut diagnostics = Vec::new();

      for (extra_key, extra_value) in table.entries().read().iter() {
        let extra_name = extra_key.value();

        let location = format!("project.optional-dependencies.{extra_name}");

        if ExtraName::from_str(extra_name).is_err() {
          diagnostics.push(Diagnostic::error(
            format!(
              "`{location}` key `{extra_name}` must be a valid PEP 508 extra name"
            ),
            extra_key.span(content),
          ));

          continue;
        }

        let Some(array) = extra_value.as_array() else {
          diagnostics.push(Diagnostic::error(
            format!("`{location}` must be an array of PEP 508 strings"),
            extra_value.span(content),
          ));

          continue;
        };

        for (index, item) in array.items().read().iter().enumerate() {
          let item_location = format!("{location}[{index}]");

          let Some(string) = item.as_str() else {
            diagnostics.push(Diagnostic::error(
              format!("`{item_location}` must be a string"),
              item.span(content),
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
                      "`{item_location}` package name `{raw_name}` must be normalized (use `{normalized}`)"
                    ),
                    item.span(content),
                  ));
                }
              }
            }
            Err(error) => diagnostics.push(Diagnostic::error(
              format!(
                "`{item_location}` item `{value}` is not a valid PEP 508 dependency: {}",
                error.message.to_string().to_lowercase()
              ),
              item.span(content),
            )),
          }
        }
      }

      diagnostics
    }
  }
}
