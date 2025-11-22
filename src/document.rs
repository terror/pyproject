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
      "
      [project]
      name = \"demo\"
      "
    };

    let document = Document::from(content);

    assert_eq!(document.content.to_string(), content);
  }

  #[test]
  fn apply_change() {
    let mut document = Document::from(indoc! {
      "
      [project]
      name = \"demo\"
      "
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
}
