use super::*;

pub(crate) struct ProjectUrlsRule;

impl Rule for ProjectUrlsRule {
  fn display_name(&self) -> &'static str {
    "Project URLs"
  }

  fn id(&self) -> &'static str {
    "project-urls"
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

    let Some(urls) = project.try_get("urls").ok() else {
      return Vec::new();
    };

    let Some(table) = urls.as_table() else {
      return vec![self.diagnostic(lsp::Diagnostic {
        message: "`project.urls` must be a table of string URLs".to_string(),
        range: urls.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })];
    };

    let mut diagnostics = Vec::new();

    for (key, value) in table.entries().read().iter() {
      if let Some(diagnostic) = self.validate_label(document, key) {
        diagnostics.push(diagnostic);
      }

      diagnostics.extend(self.validate_value(document, key.value(), value));
    }

    diagnostics
  }
}

impl ProjectUrlsRule {
  const MAX_LABEL_LENGTH: usize = 32;

  fn is_browsable_scheme(scheme: &str) -> bool {
    matches!(scheme, "http" | "https")
  }

  fn key_range(key: &Key, content: &Rope) -> lsp::Range {
    let range = key.text_ranges().next().unwrap_or_default();

    lsp::Range {
      start: content.byte_to_lsp_position(range.start().into()),
      end: content.byte_to_lsp_position(range.end().into()),
    }
  }

  fn validate_label(
    &self,
    document: &Document,
    key: &Key,
  ) -> Option<lsp::Diagnostic> {
    let label = key.value();

    if label.chars().count() > Self::MAX_LABEL_LENGTH {
      Some(self.diagnostic(lsp::Diagnostic {
        message: format!(
          "`project.urls` label `{label}` must be {} characters or fewer",
          Self::MAX_LABEL_LENGTH,
        ),
        range: Self::key_range(key, &document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }))
    } else {
      None
    }
  }

  fn validate_value(
    &self,
    document: &Document,
    label: &str,
    node: &Node,
  ) -> Vec<lsp::Diagnostic> {
    match node {
      Node::Str(string) => {
        let value = string.value();

        if value.trim().is_empty() {
          vec![self.diagnostic(lsp::Diagnostic {
            message: format!(
              "`project.urls` entry `{label}` must not be empty"
            ),
            range: node.range(&document.content),
            severity: Some(lsp::DiagnosticSeverity::ERROR),
            ..Default::default()
          })]
        } else {
          self.validate_url(document, label, node, value)
        }
      }
      _ => vec![self.diagnostic(lsp::Diagnostic {
        message: format!(
          "`project.urls` entry `{label}` must be a string URL"
        ),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })],
    }
  }

  fn validate_url(
    &self,
    document: &Document,
    label: &str,
    node: &Node,
    value: &str,
  ) -> Vec<lsp::Diagnostic> {
    match lsp::Url::parse(value) {
      Ok(url) if Self::is_browsable_scheme(url.scheme()) => Vec::new(),
      Ok(_) => vec![self.diagnostic(lsp::Diagnostic {
        message: format!(
          "`project.urls` entry `{label}` must use an `http` or `https` URL"
        ),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })],
      Err(error) => vec![self.diagnostic(lsp::Diagnostic {
        message: format!(
          "`project.urls` entry `{label}` must be a valid URL: {error}"
        ),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })],
    }
  }
}
