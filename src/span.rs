use super::*;

pub(crate) trait Span {
  fn span(&self, content: &Rope) -> lsp::Range;
}

impl Span for Key {
  fn span(&self, content: &Rope) -> lsp::Range {
    let range = self.text_ranges().next().unwrap();

    lsp::Range {
      start: content.byte_to_lsp_position(range.start().into()),
      end: content.byte_to_lsp_position(range.end().into()),
    }
  }
}

impl Span for Node {
  fn span(&self, content: &Rope) -> lsp::Range {
    let range = self.text_ranges(false).next().unwrap();

    lsp::Range {
      start: content.byte_to_lsp_position(range.start().into()),
      end: content.byte_to_lsp_position(range.end().into()),
    }
  }
}

impl Span for TextRange {
  fn span(&self, content: &Rope) -> lsp::Range {
    lsp::Range {
      start: content.byte_to_lsp_position(self.start().into()),
      end: content.byte_to_lsp_position(self.end().into()),
    }
  }
}
