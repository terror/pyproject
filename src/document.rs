use super::*;

#[allow(unused)]
#[derive(Debug)]
pub(crate) struct Document {
  pub(crate) content: Rope,
  pub(crate) uri: lsp::Url,
  pub(crate) version: i32,
}

impl TryFrom<lsp::DidOpenTextDocumentParams> for Document {
  type Error = Error;

  fn try_from(params: lsp::DidOpenTextDocumentParams) -> Result<Self> {
    let lsp::TextDocumentItem {
      text, uri, version, ..
    } = params.text_document;

    Ok(Self {
      content: Rope::from_str(&text),
      uri,
      version,
    })
  }
}

impl Document {
  /// Applies incremental edits from the client.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] if tree-sitter fails to parse the updated document.
  pub(crate) fn apply_change(
    &mut self,
    params: lsp::DidChangeTextDocumentParams,
  ) -> Result {
    let lsp::DidChangeTextDocumentParams {
      content_changes,
      text_document: lsp::VersionedTextDocumentIdentifier { version, .. },
      ..
    } = params;

    self.version = version;

    for change in content_changes {
      self.content.apply_edit(&self.content.build_edit(&change));
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    pretty_assertions::{assert_eq, assert_ne},
  };

  fn document(content: &str) -> Document {
    Document::try_from(lsp::DidOpenTextDocumentParams {
      text_document: lsp::TextDocumentItem {
        language_id: "toml".to_string(),
        text: content.to_string(),
        uri: lsp::Url::parse("file:///pyproject.toml").unwrap(),
        version: 1,
      },
    })
    .unwrap()
  }

  #[test]
  fn create_document() {
    let content = indoc! {
      "
      [project]
      name = \"demo\"
      "
    };

    let document = document(content);

    assert_eq!(document.content.to_string(), content);
  }

  #[test]
  fn apply_change() {
    let mut document = document(indoc! {
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

    document.apply_change(change).unwrap();

    assert_ne!(document.content.to_string(), original_content);

    assert_eq!(
      document.content.to_string(),
      "[project]\nname = \"example\""
    );
  }
}
