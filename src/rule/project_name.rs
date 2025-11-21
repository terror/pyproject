use {super::*, regex::Regex, std::sync::OnceLock};

pub(crate) struct ProjectNameRule;

impl Rule for ProjectNameRule {
  fn display_name(&self) -> &'static str {
    "Project Name"
  }

  fn id(&self) -> &'static str {
    "project-name"
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

    let name = project.try_get("name").ok();

    let diagnostic = match name {
      Some(name) if !name.is_str() => Some(self.diagnostic(lsp::Diagnostic {
        message: "`project.name` must be a string".to_string(),
        range: name.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })),
      Some(ref name @ Node::Str(ref string)) => {
        let value = string.value();

        if value.is_empty() {
          Some(self.diagnostic(lsp::Diagnostic {
            message: "`project.name` must not be empty".to_string(),
            range: name.range(&document.content),
            severity: Some(lsp::DiagnosticSeverity::ERROR),
            ..Default::default()
          }))
        } else {
          let normalized = Self::normalize(value);

          if normalized != value {
            Some(self.diagnostic(lsp::Diagnostic {
              message: format!(
                "`project.name` must be PEP 503 normalized (use \"{normalized}\")"
              ),
              range: name.range(&document.content),
              severity: Some(lsp::DiagnosticSeverity::ERROR),
              ..Default::default()
            }))
          } else {
            None
          }
        }
      }
      None => Some(self.diagnostic(lsp::Diagnostic {
        message: "missing required key `project.name`".to_string(),
        range: project.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })),
      _ => None,
    };

    diagnostic
      .map(|diagnostic| vec![diagnostic])
      .unwrap_or_default()
  }
}

impl ProjectNameRule {
  fn normalize(name: &str) -> String {
    static NORMALIZE_RE: OnceLock<Regex> = OnceLock::new();

    NORMALIZE_RE
      .get_or_init(|| Regex::new(r"[-_.]+").unwrap())
      .replace_all(name, "-")
      .to_ascii_lowercase()
  }
}
