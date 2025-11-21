use super::*;

pub(crate) struct RuleContext<'a> {
  document: &'a Document,
}

impl<'a> RuleContext<'a> {
  pub(crate) fn document(&self) -> &Document {
    self.document
  }

  pub(crate) fn new(document: &'a Document) -> Self {
    Self {
      document,
    }
  }

  pub(crate) fn tree(&self) -> &Parse {
    &self.document.tree
  }
}
