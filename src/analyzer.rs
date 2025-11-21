use super::*;

pub(crate) struct Analyzer<'a> {
  document: &'a Document
}

impl<'a> Analyzer<'a> {
  pub(crate) fn analyze(&self) -> Vec<lsp::Diagnostic> {
    todo!()
  }

  pub(crate) fn new(document: &'a Document) -> Self {
    Self { document }
  }
}
