use super::*;

pub(crate) trait NodeExt {
  fn range(&self, content: &Rope) -> lsp::Range;
}

impl NodeExt for Node {
  fn range(&self, content: &Rope) -> lsp::Range {
    let range = self.text_ranges(false).next().unwrap();

    lsp::Range {
      start: content.byte_to_lsp_position(range.start().into()),
      end: content.byte_to_lsp_position(range.end().into()),
    }
  }
}

impl NodeExt for Key {
  fn range(&self, content: &Rope) -> lsp::Range {
    let range = self.text_ranges().next().unwrap();

    lsp::Range {
      start: content.byte_to_lsp_position(range.start().into()),
      end: content.byte_to_lsp_position(range.end().into()),
    }
  }
}
