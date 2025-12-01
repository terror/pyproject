use super::*;

define_rule! {
  /// Validates `project.version` is present (unless dynamic) and PEP 440 compliant.
  ProjectVersionRule {
    id: "project-version",
    message: "invalid `project.version` value",
    run(context) {
      let Some(project) = context.get("project") else {
        return Vec::new();
      };

      let content = context.content();

      if Self::version_listed_in_dynamic(&project) {
        return Vec::new();
      }

      let diagnostic = match context.get("project.version") {
        Some(version) if !version.is_str() => Some(Diagnostic::error(
          "`project.version` must be a string",
          version.span(content),
        )),
        Some(ref version @ Node::Str(ref string)) => {
          let value = string.value();

          if value.is_empty() {
            Some(Diagnostic::error(
              "`project.version` must not be empty",
              version.span(content),
            ))
          } else if let Err(error) = Version::from_str(value) {
            Some(Diagnostic::error(
              error.to_string(),
              version.span(content),
            ))
          } else {
            None
          }
        }
        None => Some(Diagnostic::error(
          "missing required key `project.version`",
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

impl ProjectVersionRule {
  fn version_listed_in_dynamic(project: &Node) -> bool {
    let Some(dynamic) = project.try_get("dynamic").ok() else {
      return false;
    };

    let Some(items) = dynamic.as_array().map(|array| array.items().read())
    else {
      return false;
    };

    items.iter().any(|item| {
      item
        .as_str()
        .is_some_and(|string| string.value() == "version")
    })
  }
}
