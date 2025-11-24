use super::*;

pub(crate) struct ProjectVersionRule;

impl Rule for ProjectVersionRule {
  fn header(&self) -> &'static str {
    "invalid project.version"
  }

  fn id(&self) -> &'static str {
    "project-version"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let Some(project) = context.project() else {
      return Vec::new();
    };

    let document = context.document();

    if Self::version_listed_in_dynamic(&project) {
      return Vec::new();
    }

    let version = project.try_get("version").ok();

    let diagnostic = match version {
      Some(version) if !version.is_str() => Some(lsp::Diagnostic {
        message: "`project.version` must be a string".to_string(),
        range: version.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }),
      Some(ref version @ Node::Str(ref string)) => {
        let value = string.value();

        if value.is_empty() {
          Some(lsp::Diagnostic {
            message: "`project.version` must not be empty".to_string(),
            range: version.range(&document.content),
            severity: Some(lsp::DiagnosticSeverity::ERROR),
            ..Default::default()
          })
        } else if let Err(error) = Version::from_str(value) {
          Some(lsp::Diagnostic {
            message: error.to_string(),
            range: version.range(&document.content),
            severity: Some(lsp::DiagnosticSeverity::ERROR),
            ..Default::default()
          })
        } else {
          None
        }
      }
      None => Some(lsp::Diagnostic {
        message: "missing required key `project.version`".to_string(),
        range: project.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }),
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
