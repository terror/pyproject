use super::*;

pub(crate) struct ProjectVersionRule;

impl Rule for ProjectVersionRule {
  fn display_name(&self) -> &'static str {
    "Project Version"
  }

  fn id(&self) -> &'static str {
    "project-version"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let document = context.document();

    let tree = context.tree().clone().into_dom();

    let Some(project) = tree.try_get("project").ok() else {
      return Vec::new();
    };

    let version = project.try_get("version").ok();

    if let Some(version) = version {
      return self.diagnostics_for_version(document, version);
    }

    if Self::version_listed_in_dynamic(&project) {
      return Vec::new();
    }

    vec![self.diagnostic(lsp::Diagnostic {
      message: "missing required key `project.version`".to_string(),
      range: project.range(&document.content),
      severity: Some(lsp::DiagnosticSeverity::ERROR),
      ..Default::default()
    })]
  }
}

impl ProjectVersionRule {
  fn diagnostics_for_version(
    &self,
    document: &Document,
    version: Node,
  ) -> Vec<lsp::Diagnostic> {
    match &version {
      Node::Str(string) if string.value().is_empty() => {
        vec![self.diagnostic(lsp::Diagnostic {
          message: "`project.version` must not be empty".to_string(),
          range: version.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        })]
      }
      Node::Str(_) => Vec::new(),
      _ => vec![self.diagnostic(lsp::Diagnostic {
        message: "`project.version` must be a string".to_string(),
        range: version.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })],
    }
  }

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
