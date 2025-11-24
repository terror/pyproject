use super::*;

#[allow(unused)]
#[derive(Debug)]
pub(crate) struct Document {
  pub(crate) content: Rope,
  pub(crate) tree: Parse,
  pub(crate) uri: lsp::Url,
  pub(crate) version: i32,
}

#[cfg(test)]
impl From<&str> for Document {
  fn from(value: &str) -> Self {
    Self {
      content: value.into(),
      tree: parse(value),
      uri: lsp::Url::from_file_path(env::temp_dir().join("pyproject.toml"))
        .unwrap(),
      version: 1,
    }
  }
}

#[cfg(test)]
impl From<lsp::Url> for Document {
  fn from(value: lsp::Url) -> Self {
    Self {
      content: "".into(),
      tree: parse(""),
      uri: value,
      version: 1,
    }
  }
}

impl From<lsp::DidOpenTextDocumentParams> for Document {
  fn from(params: lsp::DidOpenTextDocumentParams) -> Self {
    let lsp::TextDocumentItem {
      text, uri, version, ..
    } = params.text_document;

    Self {
      content: Rope::from_str(&text),
      tree: parse(&text),
      uri,
      version,
    }
  }
}

impl Document {
  pub(crate) fn apply_change(
    &mut self,
    params: lsp::DidChangeTextDocumentParams,
  ) {
    let lsp::DidChangeTextDocumentParams {
      content_changes,
      text_document: lsp::VersionedTextDocumentIdentifier { version, .. },
      ..
    } = params;

    self.version = version;

    for change in content_changes {
      self.content.apply_edit(&self.content.build_edit(&change));
    }

    self.tree = parse(&self.content.to_string());
  }

  pub(crate) fn resolve_path(&self, path: &str) -> Option<PathBuf> {
    let Ok(mut document_path) = self.uri.to_file_path() else {
      return None;
    };

    let path = Path::new(path);

    if path.is_absolute() {
      return Some(path.to_path_buf());
    }

    document_path.pop();

    Some(document_path.join(path))
  }

  pub(crate) fn root(&self) -> Option<PathBuf> {
    let Ok(mut path) = self.uri.to_file_path() else {
      return None;
    };

    path.pop();

    Some(path)
  }

  pub(crate) fn validate_relative_path(
    &self,
    path: &str,
    setting: &str,
    node: &Node,
  ) -> Result<PathBuf, Vec<Diagnostic>> {
    let range = node.span(&self.content);

    let make_error = |message: String| {
      Diagnostic::new(message, range, lsp::DiagnosticSeverity::ERROR)
    };

    if path.trim().is_empty() {
      return Err(vec![make_error(format!(
        "file path for `{setting}` must not be empty"
      ))]);
    }

    let mut diagnostics = Vec::new();

    let path_ref = Path::new(path);

    if path_ref.is_absolute() {
      diagnostics.push(make_error(format!(
        "file path for `{setting}` must be relative"
      )));
    }

    let Some(resolved_path) = self.resolve_path(path) else {
      diagnostics.push(make_error(format!(
        "file `{path}` for `{setting}` does not exist"
      )));

      return Err(diagnostics);
    };

    if !resolved_path.exists() {
      diagnostics.push(make_error(format!(
        "file `{path}` for `{setting}` does not exist"
      )));
    } else if let Err(error) = fs::read_to_string(&resolved_path) {
      diagnostics.push(make_error(format!(
        "file `{path}` for `{setting}` must be valid UTF-8 text ({error})"
      )));
    }

    if diagnostics.is_empty() {
      Ok(resolved_path)
    } else {
      Err(diagnostics)
    }
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    pretty_assertions::{assert_eq, assert_ne},
  };

  #[test]
  fn create_document() {
    let content = indoc! {
      r#"
      [project]
      name = "demo"
      "#
    };

    let document = Document::from(content);

    assert_eq!(document.content.to_string(), content);
  }

  #[test]
  fn apply_change() {
    let mut document = Document::from(indoc! {
      r#"
      [project]
      name = "demo"
      "#
    });

    let original_content = document.content.to_string();

    let change = lsp::DidChangeTextDocumentParams {
      text_document: lsp::VersionedTextDocumentIdentifier {
        uri: lsp::Url::parse("file:///pyproject.toml").unwrap(),
        version: 2,
      },
      content_changes: vec![lsp::TextDocumentContentChangeEvent {
        range: Some((1, 7, 1, 14).range()),
        range_length: None,
        text: "\"example\"".to_string(),
      }],
    };

    document.apply_change(change);

    assert_ne!(document.content.to_string(), original_content);

    assert_eq!(
      document.content.to_string(),
      "[project]\nname = \"example\""
    );
  }

  #[test]
  #[cfg(unix)]
  fn resolve_path_relative() {
    let document = Document::from(
      lsp::Url::from_file_path("/home/user/project/pyproject.toml").unwrap(),
    );

    assert_eq!(
      document.resolve_path("README.md").unwrap(),
      PathBuf::from("/home/user/project/README.md")
    );
  }

  #[test]
  #[cfg(unix)]
  fn resolve_path_relative_subdirectory() {
    let document = Document::from(
      lsp::Url::from_file_path("/home/user/project/pyproject.toml").unwrap(),
    );

    assert_eq!(
      document.resolve_path("docs/guide.md").unwrap(),
      PathBuf::from("/home/user/project/docs/guide.md")
    );
  }

  #[test]
  #[cfg(unix)]
  fn resolve_path_relative_parent() {
    let document = Document::from(
      lsp::Url::from_file_path("/home/user/project/pyproject.toml").unwrap(),
    );

    assert_eq!(
      document.resolve_path("../LICENSE").unwrap(),
      PathBuf::from("/home/user/project/../LICENSE")
    );
  }

  #[test]
  #[cfg(unix)]
  fn resolve_path_absolute() {
    let document = Document::from(
      lsp::Url::from_file_path("/home/user/project/pyproject.toml").unwrap(),
    );

    assert_eq!(
      document.resolve_path("/etc/config").unwrap(),
      PathBuf::from("/etc/config")
    );
  }

  #[test]
  #[cfg(unix)]
  fn resolve_path_current_directory() {
    let document = Document::from(
      lsp::Url::from_file_path("/home/user/project/pyproject.toml").unwrap(),
    );

    assert_eq!(
      document.resolve_path("./README.md").unwrap(),
      PathBuf::from("/home/user/project/./README.md")
    );
  }

  #[test]
  #[cfg(windows)]
  fn resolve_path_windows_absolute() {
    let document = Document::from(
      lsp::Url::from_file_path("C:\\Users\\user\\project\\pyproject.toml")
        .unwrap(),
    );

    assert_eq!(
      document.resolve_path("C:\\config\\file.txt").unwrap(),
      PathBuf::from("C:\\config\\file.txt")
    );
  }

  #[test]
  #[cfg(windows)]
  fn resolve_path_windows_relative() {
    let document = Document::from(
      lsp::Url::from_file_path("C:\\Users\\user\\project\\pyproject.toml")
        .unwrap(),
    );

    assert_eq!(
      document.resolve_path("README.md").unwrap(),
      PathBuf::from("C:\\Users\\user\\project\\README.md")
    );
  }

  #[test]
  #[cfg(unix)]
  fn root() {
    let document = Document::from(
      lsp::Url::from_file_path("/home/user/project/pyproject.toml").unwrap(),
    );

    assert_eq!(
      document.root().unwrap(),
      PathBuf::from("/home/user/project")
    );
  }

  #[test]
  #[cfg(unix)]
  fn root_nested() {
    let document = Document::from(
      lsp::Url::from_file_path("/home/user/project/subdir/pyproject.toml")
        .unwrap(),
    );

    assert_eq!(
      document.root().unwrap(),
      PathBuf::from("/home/user/project/subdir")
    );
  }

  #[test]
  #[cfg(unix)]
  fn root_at_filesystem_root() {
    let document =
      Document::from(lsp::Url::from_file_path("/pyproject.toml").unwrap());

    assert_eq!(document.root().unwrap(), PathBuf::from("/"));
  }

  #[test]
  #[cfg(windows)]
  fn root_windows() {
    let document = Document::from(
      lsp::Url::from_file_path("C:\\Users\\user\\project\\pyproject.toml")
        .unwrap(),
    );

    assert_eq!(
      document.root().unwrap(),
      PathBuf::from("C:\\Users\\user\\project")
    );
  }

  #[test]
  #[cfg(windows)]
  fn root_windows_nested() {
    let document = Document::from(
      lsp::Url::from_file_path(
        "C:\\Users\\user\\project\\subdir\\pyproject.toml",
      )
      .unwrap(),
    );

    assert_eq!(
      document.root().unwrap(),
      PathBuf::from("C:\\Users\\user\\project\\subdir")
    );
  }

  #[test]
  #[cfg(windows)]
  fn root_windows_drive_root() {
    let document =
      Document::from(lsp::Url::from_file_path("C:\\pyproject.toml").unwrap());

    assert_eq!(document.root().unwrap(), PathBuf::from("C:\\"));
  }
}
