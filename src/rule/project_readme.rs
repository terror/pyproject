use super::*;

pub(crate) struct ProjectReadmeRule;

impl Rule for ProjectReadmeRule {
  fn display_name(&self) -> &'static str {
    "Project Readme"
  }

  fn id(&self) -> &'static str {
    "project-readme"
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

    let Some(readme) = project.try_get("readme").ok() else {
      return Vec::new();
    };

    match &readme {
      Node::Str(string) => self.check_path(document, string.value(), &readme),
      Node::Table(_) => self.check_table(document, &readme),
      _ => vec![self.diagnostic(lsp::Diagnostic {
        message: "`project.readme` must be a string or table".to_string(),
        range: readme.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })],
    }
  }
}

impl ProjectReadmeRule {
  fn check_path(
    &self,
    document: &Document,
    path: &str,
    node: &Node,
  ) -> Vec<lsp::Diagnostic> {
    if path.trim().is_empty() {
      return vec![self.diagnostic(lsp::Diagnostic {
        message: "file path for `project.readme` must not be empty".to_string(),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })];
    }

    let Some(resolved_path) = Self::resolve_path(document, path) else {
      return Vec::new();
    };

    if resolved_path.exists() {
      Vec::new()
    } else {
      vec![self.diagnostic(lsp::Diagnostic {
        message: format!("file `{path}` for `project.readme` does not exist"),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })]
    }
  }

  fn check_table(
    &self,
    document: &Document,
    readme: &Node,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    match readme.try_get("content-type") {
      Ok(content_type) => {
        if !content_type.is_str() {
          diagnostics.push(self.diagnostic(lsp::Diagnostic {
            message:
              "`project.readme.content-type` must be a string".to_string(),
            range: content_type.range(&document.content),
            severity: Some(lsp::DiagnosticSeverity::ERROR),
            ..Default::default()
          }));
        }
      }
      Err(_) => diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message:
          "missing required key `project.readme.content-type`".to_string(),
        range: readme.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })),
    }

    if let Ok(file) = readme.try_get("file") {
      match file {
        Node::Str(ref string) => {
          diagnostics.extend(self.check_path(document, string.value(), &file));
        }
        _ => diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: "`project.readme.file` must be a string".to_string(),
          range: file.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        })),
      }
    }

    diagnostics
  }

  fn resolve_path(document: &Document, path: &str) -> Option<PathBuf> {
    let Ok(mut document_path) = document.uri.to_file_path() else {
      return None;
    };

    let path = Path::new(path);

    if path.is_absolute() {
      return Some(path.to_path_buf());
    }

    document_path.pop();

    Some(document_path.join(path))
  }
}
