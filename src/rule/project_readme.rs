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
      Node::Str(string) => {
        self.check_readme_string(document, string.value(), &readme)
      }
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
  const KNOWN_README_EXTENSIONS: [&'static str; 2] = ["md", "rst"];

  fn check_readme_string(
    &self,
    document: &Document,
    path: &str,
    node: &Node,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = self.validate_path(document, path, node);

    if !Self::has_known_extension(path) {
      diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message: "`project.readme` must point to a `.md` or `.rst` file when specified as a string".to_string(),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }));
    }

    diagnostics
  }

  fn check_table(
    &self,
    document: &Document,
    readme: &Node,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    let file = readme.try_get("file").ok();
    let text = readme.try_get("text").ok();

    match (file.as_ref(), text.as_ref()) {
      (Some(_), Some(_)) => diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message: "`project.readme` must specify only one of `file` or `text`".to_string(),
        range: readme.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })),
      (None, None) => diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message:
          "missing required key `project.readme.file` or `project.readme.text`"
            .to_string(),
        range: readme.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })),
      _ => {}
    }

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

    if let Some(ref file) = file {
      match file {
        Node::Str(string) => {
          diagnostics.extend(self.validate_path(
            document,
            string.value(),
            file,
          ));
        }
        _ => diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: "`project.readme.file` must be a string".to_string(),
          range: file.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        })),
      }
    }

    match text {
      Some(text) if !text.is_str() => {
        diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: "`project.readme.text` must be a string".to_string(),
          range: text.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        }));
      }
      _ => {}
    }

    diagnostics
  }

  fn has_known_extension(path: &str) -> bool {
    let Some(extension) =
      Path::new(path).extension().and_then(|ext| ext.to_str())
    else {
      return false;
    };

    Self::KNOWN_README_EXTENSIONS
      .iter()
      .any(|known| extension.eq_ignore_ascii_case(known))
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

  fn validate_path(
    &self,
    document: &Document,
    path: &str,
    node: &Node,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    if path.trim().is_empty() {
      diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message: "file path for `project.readme` must not be empty".to_string(),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }));

      return diagnostics;
    }

    if Path::new(path).is_absolute() {
      diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message: "file path for `project.readme` must be relative".to_string(),
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
        message: format!("file `{path}` for `project.readme` does not exist"),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }));
    }

    diagnostics
  }
}
