#![cfg(test)]

use super::*;

pub(crate) trait Range {
  fn range(self) -> lsp::Range;
}

impl Range for (u32, u32, u32, u32) {
  fn range(self) -> lsp::Range {
    lsp::Range {
      start: lsp::Position {
        line: self.0,
        character: self.1,
      },
      end: lsp::Position {
        line: self.2,
        character: self.3,
      },
    }
  }
}
