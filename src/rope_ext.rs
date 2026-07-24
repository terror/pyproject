use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit<'a> {
  pub end_char: usize,
  pub start_char: usize,
  pub text: &'a str,
}

pub trait RopeExt {
  fn apply_edit(&mut self, edit: &Edit);
  fn build_edit<'a>(
    &self,
    change: &'a lsp::TextDocumentContentChangeEvent,
  ) -> Edit<'a>;
  fn byte_to_lsp_position(&self, byte: usize) -> lsp::Position;
  fn lsp_position_to_char(&self, position: lsp::Position) -> usize;
}

impl RopeExt for Rope {
  fn apply_edit(&mut self, edit: &Edit) {
    self.remove(edit.start_char..edit.end_char);

    if !edit.text.is_empty() {
      self.insert(edit.start_char, edit.text);
    }
  }

  fn build_edit<'a>(
    &self,
    change: &'a lsp::TextDocumentContentChangeEvent,
  ) -> Edit<'a> {
    let text = change.text.as_str();

    let range = change.range.unwrap_or_else(|| lsp::Range {
      start: self.byte_to_lsp_position(0),
      end: self.byte_to_lsp_position(self.len_bytes()),
    });

    let (start, old_end) = (
      self.lsp_position_to_char(range.start),
      self.lsp_position_to_char(range.end),
    );

    Edit {
      end_char: old_end,
      start_char: start,
      text,
    }
  }

  fn byte_to_lsp_position(&self, byte: usize) -> lsp::Position {
    let line = self.byte_to_line(byte);

    let line_char = self.line_to_char(line);
    let line_utf16_cu = self.char_to_utf16_cu(line_char);

    let char = self.byte_to_char(byte);
    let char_utf16_cu = self.char_to_utf16_cu(char);

    lsp::Position::new(
      u32::try_from(line).expect("line index exceeds u32::MAX"),
      u32::try_from(char_utf16_cu - line_utf16_cu)
        .expect("character offset exceeds u32::MAX"),
    )
  }

  fn lsp_position_to_char(&self, position: lsp::Position) -> usize {
    let row = position.line as usize;

    let row_char = self.line_to_char(row);

    self.utf16_cu_to_char(
      self.char_to_utf16_cu(row_char) + position.character as usize,
    )
  }
}

#[cfg(test)]
mod tests {
  use {super::*, pretty_assertions::assert_eq};

  fn change(
    text: &str,
    range: lsp::Range,
  ) -> lsp::TextDocumentContentChangeEvent {
    lsp::TextDocumentContentChangeEvent {
      range: Some(range),
      range_length: None,
      text: text.into(),
    }
  }

  #[test]
  fn apply_insert_into_empty_document() {
    let mut rope = Rope::from_str("");

    let change = change("🧪\nnew", (0, 0, 0, 0).range());

    let edit = rope.build_edit(&change);

    assert_eq!(
      edit,
      Edit {
        start_char: 0,
        end_char: 0,
        text: "🧪\nnew",
      }
    );

    rope.apply_edit(&edit);

    assert_eq!(rope.to_string(), "🧪\nnew");
  }

  #[test]
  fn apply_insert_edit_updates_rope_contents() {
    let mut rope = Rope::from_str("hello world");

    let change = change("rope", (0, 6, 0, 11).range());

    let edit = rope.build_edit(&change);

    assert_eq!(
      edit,
      Edit {
        start_char: 6,
        end_char: 11,
        text: "rope",
      }
    );

    rope.apply_edit(&edit);

    assert_eq!(rope.to_string(), "hello rope");
  }

  #[test]
  fn apply_insert_edit_respects_utf16_columns() {
    let mut rope = Rope::from_str("ab");

    let change = change("🧪", (0, 1, 0, 1).range());

    let edit = rope.build_edit(&change);

    assert_eq!(
      edit,
      Edit {
        start_char: 1,
        end_char: 1,
        text: "🧪",
      }
    );

    rope.apply_edit(&edit);

    assert_eq!(rope.to_string(), "a🧪b");
  }

  #[test]
  fn apply_delete_edit_respects_utf16_columns() {
    let mut rope = Rope::from_str("a😊b");

    let change = change("", (0, 1, 0, 3).range());

    let edit = rope.build_edit(&change);

    assert_eq!(
      edit,
      Edit {
        start_char: 1,
        end_char: 2,
        text: "",
      }
    );

    rope.apply_edit(&edit);

    assert_eq!(rope.to_string(), "ab");
  }

  #[test]
  fn lsp_round_trip_handles_utf16_columns() {
    let rope = Rope::from_str("a😊b\nsecond");

    let position = rope.byte_to_lsp_position(5);

    assert_eq!(position, lsp::Position::new(0, 3));

    assert_eq!(rope.lsp_position_to_char(position), 2);
  }

  #[test]
  fn replacement_across_surrogates_is_consistent() {
    let mut rope = Rope::from_str("foo😊bar");

    let change = change("🧪", (0, 3, 0, 5).range());

    let edit = rope.build_edit(&change);

    assert_eq!(
      edit,
      Edit {
        start_char: 3,
        end_char: 4,
        text: "🧪",
      }
    );

    rope.apply_edit(&edit);

    assert_eq!(rope.to_string(), "foo🧪bar");
  }

  #[test]
  fn multiline_edit_handles_utf16_offsets() {
    let mut rope = Rope::from_str("foo😊\nbar");

    let change = change("XX", (0, 2, 1, 1).range());

    let edit = rope.build_edit(&change);

    assert_eq!(
      edit,
      Edit {
        start_char: 2,
        end_char: 6,
        text: "XX",
      }
    );

    rope.apply_edit(&edit);

    assert_eq!(rope.to_string(), "foXXar");
  }

  #[test]
  fn append_beyond_eof_updates_point() {
    let mut rope = Rope::from_str("hi");

    let change = change("🧪\nnew", (0, 2, 0, 2).range());

    let edit = rope.build_edit(&change);

    assert_eq!(
      edit,
      Edit {
        start_char: 2,
        end_char: 2,
        text: "🧪\nnew",
      }
    );

    rope.apply_edit(&edit);

    assert_eq!(rope.to_string(), "hi🧪\nnew");
  }

  #[test]
  fn replace_entire_document_via_full_range() {
    let mut rope = Rope::from_str("foo😊bar");

    let change = lsp::TextDocumentContentChangeEvent {
      range: None,
      range_length: None,
      text: "🧪baz".into(),
    };

    let edit = rope.build_edit(&change);

    assert_eq!(
      edit,
      Edit {
        start_char: 0,
        end_char: 7,
        text: "🧪baz",
      }
    );

    rope.apply_edit(&edit);

    assert_eq!(rope.to_string(), "🧪baz");
  }
}
