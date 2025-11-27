use super::*;

pub(crate) struct ProjectVersionRule;

impl Rule for ProjectVersionRule {
  fn id(&self) -> &'static str {
    "project-version"
  }

  fn message(&self) -> &'static str {
    "invalid `project.version` value"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(project) = context.project() else {
      return Vec::new();
    };

    let document = context.document();

    if Self::version_listed_in_dynamic(&project) {
      return Vec::new();
    }

    let diagnostic = match context.get("project.version") {
      Some(version) if !version.is_str() => Some(Diagnostic::error(
        "`project.version` must be a string",
        version.span(&document.content),
      )),
      Some(ref version @ Node::Str(ref string)) => {
        let value = string.value();

        if value.is_empty() {
          Some(Diagnostic::error(
            "`project.version` must not be empty",
            version.span(&document.content),
          ))
        } else if let Err(error) = Version::from_str(value) {
          Some(Diagnostic::error(
            error.to_string(),
            version.span(&document.content),
          ))
        } else {
          None
        }
      }
      None => Some(Diagnostic::error(
        "missing required key `project.version`",
        project.span(&document.content),
      )),
      _ => None,
    };

    diagnostic
      .map(|diagnostic| vec![diagnostic])
      .unwrap_or_default()
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
