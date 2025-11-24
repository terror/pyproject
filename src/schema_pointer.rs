use super::*;

#[derive(Debug)]
pub(crate) struct PointerMap<'a> {
  document: &'a Document,
  ranges: HashMap<String, TextRange>,
}

impl<'a> PointerMap<'a> {
  pub(crate) fn build(document: &'a Document, root: &Node) -> (Value, Self) {
    let instance = serde_json::to_value(root).unwrap_or_else(|error| {
      panic!("failed to convert document to JSON: {error}")
    });

    let mut map = Self {
      document,
      ranges: HashMap::new(),
    };

    map.populate(root, String::new(), None);

    (instance, map)
  }

  pub(crate) fn diagnostic(&self, error: ValidationError) -> Diagnostic {
    Diagnostic::new(
      SchemaError(&error).to_string(),
      self.range_for_error(&error),
      lsp::DiagnosticSeverity::ERROR,
    )
  }

  fn diagnostic_range(&self, pointer: Option<String>) -> TextRange {
    pointer
      .as_deref()
      .and_then(|pointer| self.ranges.get(pointer))
      .copied()
      .unwrap_or_else(|| self.root_range())
  }

  fn empty_range() -> TextRange {
    TextRange::empty(TextSize::from(0))
  }

  fn encode_segment(segment: &str) -> String {
    let mut encoded = String::with_capacity(segment.len());

    for ch in segment.chars() {
      match ch {
        '~' => encoded.push_str("~0"),
        '/' => encoded.push_str("~1"),
        _ => encoded.push(ch),
      }
    }

    encoded
  }

  fn join(parent: &str, segment: &str) -> String {
    if parent.is_empty() {
      format!("/{}", Self::encode_segment(segment))
    } else {
      format!("{}/{}", parent, Self::encode_segment(segment))
    }
  }

  fn node_range(node: &Node, key: Option<&Key>) -> TextRange {
    let base = node
      .text_ranges(false)
      .next()
      .unwrap_or_else(Self::empty_range);

    match key.and_then(|key| key.text_ranges().next()) {
      Some(key_range) => base.cover(key_range),
      None => base,
    }
  }

  fn pointer_for_error(error: &ValidationError) -> Option<String> {
    match error.kind() {
      ValidationErrorKind::AdditionalProperties { unexpected }
      | ValidationErrorKind::UnevaluatedItems { unexpected }
      | ValidationErrorKind::UnevaluatedProperties { unexpected } => Some(
        PointerMap::join(error.instance_path().as_str(), unexpected.first()?),
      ),
      _ => Some(error.instance_path().as_str().to_string()),
    }
  }

  pub(crate) fn pointer_for_position(
    &self,
    position: lsp::Position,
  ) -> Option<String> {
    let byte = self
      .document
      .content
      .char_to_byte(self.document.content.lsp_position_to_char(position));

    let offset = TextSize::try_from(byte).ok()?;

    self
      .ranges
      .iter()
      .filter(|(_, range)| range.contains_inclusive(offset))
      .min_by_key(|(_, range)| range.len())
      .map(|(pointer, _)| pointer.clone())
  }

  fn populate(&mut self, node: &Node, pointer: String, key: Option<&Key>) {
    let range = Self::node_range(node, key);

    self.ranges.insert(pointer.clone(), range);

    match node {
      Node::Table(table) => {
        for (entry_key, value) in table.entries().read().iter() {
          self.populate(
            value,
            Self::join(&pointer, entry_key.value()),
            Some(entry_key),
          );
        }
      }
      Node::Array(array) => {
        for (idx, value) in array.items().read().iter().enumerate() {
          self.populate(value, Self::join(&pointer, &idx.to_string()), None);
        }
      }
      _ => {}
    }
  }

  fn range_for_error(&self, error: &ValidationError) -> lsp::Range {
    self
      .diagnostic_range(Self::pointer_for_error(error))
      .span(&self.document.content)
  }

  pub(crate) fn range_for_pointer(&self, pointer: &str) -> TextRange {
    if let Some(range) = self.ranges.get(pointer) {
      return *range;
    }

    let mut current = pointer;

    while let Some(idx) = current.rfind('/') {
      current = &current[..idx];

      if let Some(range) = self.ranges.get(current) {
        return *range;
      }
    }

    self.root_range()
  }

  fn root_range(&self) -> TextRange {
    self
      .ranges
      .get("")
      .copied()
      .unwrap_or_else(Self::empty_range)
  }
}

#[cfg(test)]
mod tests {
  use {super::*, indoc::indoc, pretty_assertions::assert_eq};

  #[test]
  fn pointer_for_position_returns_most_specific_pointer() {
    let document = Document::from(indoc! {
      r#"
      [tool]
      name = "demo"
      items = ["one", "two"]
      "#
    });

    let dom = document.tree.clone().into_dom();

    let (_, pointers) = PointerMap::build(&document, &dom);

    assert_eq!(
      pointers.pointer_for_position(lsp::Position::new(1, 9)),
      Some("/tool/name".to_string())
    );

    assert_eq!(
      pointers.pointer_for_position(lsp::Position::new(2, 18)),
      Some("/tool/items/1".to_string())
    );
  }

  #[test]
  fn range_for_pointer_falls_back_to_nearest_parent() {
    let document = Document::from(indoc! {
      r#"
      [tool]
      name = "demo"
      items = ["one", "two"]
      "#
    });

    let dom = document.tree.clone().into_dom();

    let (_, pointers) = PointerMap::build(&document, &dom);

    assert_eq!(
      pointers
        .range_for_pointer("/tool/items/missing")
        .span(&document.content),
      (2, 0, 2, 22).range()
    );
  }

  #[test]
  fn pointer_segments_are_encoded() {
    let document = Document::from(indoc! {
      r#"
      [section]
      "tilde~key" = "first"
      "slash/key" = "second"
      "#
    });

    let dom = document.tree.clone().into_dom();

    let (_, pointers) = PointerMap::build(&document, &dom);

    assert_eq!(
      pointers.pointer_for_position(lsp::Position::new(1, 16)),
      Some("/section/tilde~0key".to_string())
    );

    assert_eq!(
      pointers.pointer_for_position(lsp::Position::new(2, 16)),
      Some("/section/slash~1key".to_string())
    );
  }
}
