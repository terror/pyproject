use super::*;

#[derive(Clone, Debug)]
pub struct Quickfix {
  pub edits: Vec<lsp::TextEdit>,
  pub title: String,
}

impl Quickfix {
  pub fn replacement(
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
