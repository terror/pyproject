use super::*;

#[derive(Clone, Debug)]
pub(crate) struct Quickfix {
  pub(crate) edits: Vec<lsp::TextEdit>,
  pub(crate) title: String,
}

impl Quickfix {
  pub(crate) fn replacement(
    range: lsp::Range,
    value: &str,
    replacement: impl Into<String>,
  ) -> Self {
    let replacement = replacement.into();

    Self {
      edits: vec![lsp::TextEdit {
        range,
        new_text: replacement.clone(),
      }],
      title: format!("Replace `{value}` with `{replacement}`"),
    }
  }
}
