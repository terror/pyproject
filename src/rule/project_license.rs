use super::*;

pub(crate) struct ProjectLicenseRule;

impl Rule for ProjectLicenseRule {
  fn display_name(&self) -> &'static str {
    "Project License"
  }

  fn id(&self) -> &'static str {
    "project-license"
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

    let Some(license) = project.try_get("license").ok() else {
      return Vec::new();
    };

    match &license {
      Node::Str(string) => {
        if string.value().trim().is_empty() {
          vec![self.diagnostic(lsp::Diagnostic {
            message: "`project.license` must not be empty".to_string(),
            range: license.range(&document.content),
            severity: Some(lsp::DiagnosticSeverity::ERROR),
            ..Default::default()
          })]
        } else {
          Vec::new()
        }
      }
      Node::Table(_) => self.check_table(document, &license),
      _ => vec![self.diagnostic(lsp::Diagnostic {
        message: "`project.license` must be a string or table".to_string(),
        range: license.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })],
    }
  }
}

impl ProjectLicenseRule {
  fn check_table(
    &self,
    document: &Document,
    license: &Node,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    let file = license.try_get("file").ok();
    let text = license.try_get("text").ok();

    match (file.as_ref(), text.as_ref()) {
      (Some(_), Some(_)) => diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message: "`project.license` must specify only one of `file` or `text`"
          .to_string(),
        range: license.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })),
      (None, None) => diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message:
          "missing required key `project.license.file` or `project.license.text`"
            .to_string(),
        range: license.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })),
      _ => {}
    }

    if let Some(ref file) = file {
      match file {
        Node::Str(string) => diagnostics.extend(self.validate_path(
          document,
          string.value(),
          file,
        )),
        _ => diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: "`project.license.file` must be a string".to_string(),
          range: file.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        })),
      }
    }

    if let Some(text) = text {
      if !text.is_str() {
        diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: "`project.license.text` must be a string".to_string(),
          range: text.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        }));
      }
    }

    diagnostics
  }

  fn validate_path(
    &self,
    document: &Document,
    path: &str,
    node: &Node,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    let path_ref = Path::new(path);

    if path.trim().is_empty() {
      diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message:
          "file path for `project.license.file` must not be empty".to_string(),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }));

      return diagnostics;
    }

    if Self::path_is_rooted(path_ref) {
      diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message: "file path for `project.license.file` must be relative"
          .to_string(),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }));
    }

    let Some(resolved_path) = Self::resolve_path(document, path) else {
      return diagnostics;
    };

    if !resolved_path.exists() {
      diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message: format!(
          "file `{path}` for `project.license.file` does not exist"
        ),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }));
    }

    diagnostics
  }

  fn path_is_rooted(path: &Path) -> bool {
    path.has_root()
      || path
        .components()
        .any(|component| matches!(component, Component::Prefix(_)))
  }

  fn resolve_path(document: &Document, path: &str) -> Option<PathBuf> {
    let Ok(mut document_path) = document.uri.to_file_path() else {
      return None;
    };

    let path = Path::new(path);

    if Self::path_is_rooted(path) {
      return Some(path.to_path_buf());
    }

    document_path.pop();

    Some(document_path.join(path))
  }
}
