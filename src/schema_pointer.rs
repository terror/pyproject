use super::*;

#[derive(Debug)]
pub(crate) struct SchemaPointer<'a> {
  document: &'a Document,
  ranges: HashMap<String, TextRange>,
}

impl<'a> SchemaPointer<'a> {
  pub(crate) fn build(document: &'a Document) -> Result<(Value, Self)> {
    let root = document.tree.clone().into_dom();

    let instance = serde_json::to_value(&root)
      .map_err(|source| Error::DocumentJson { source })?;

    let ranges = iter::once((String::new(), Self::node_range(&root, None)))
      .chain(root.flat_iter().map(|(keys, node)| {
        let pointer = keys.iter().fold(String::new(), |mut pointer, key| {
          pointer.push('/');

          match key {
            KeyOrIndex::Key(key) => {
              pointer
                .push_str(&key.value().replace('~', "~0").replace('/', "~1"));
            }
            KeyOrIndex::Index(index) => pointer.push_str(&index.to_string()),
          }

          pointer
        });

        (
          pointer,
          Self::node_range(
            &node,
            keys.iter().last().and_then(KeyOrIndex::as_key),
          ),
        )
      }))
      .collect();

    Ok((instance, Self { document, ranges }))
  }

  pub(crate) fn diagnostic(&self, error: ValidationError) -> Diagnostic {
    let pointer = match error.kind() {
      ValidationErrorKind::AdditionalProperties { unexpected }
      | ValidationErrorKind::UnevaluatedItems { unexpected }
      | ValidationErrorKind::UnevaluatedProperties { unexpected } => {
        unexpected.first().map(|unexpected| {
          let parent = error.instance_path().as_str();

          let unexpected = unexpected.replace('~', "~0").replace('/', "~1");

          if parent.is_empty() {
            format!("/{unexpected}")
          } else {
            format!("{parent}/{unexpected}")
          }
        })
      }
      _ => Some(error.instance_path().as_str().to_string()),
    };

    let range = pointer.as_deref().map_or_else(
      || self.range_for_pointer(""),
      |pointer| self.range_for_pointer(pointer),
    );

    Diagnostic::error(
      SchemaError(&error).to_string(),
      range.span(&self.document.content),
    )
  }

  fn node_range(node: &Node, key: Option<&Key>) -> TextRange {
    let base = node
      .text_ranges(false)
      .next()
      .unwrap_or_else(|| TextRange::empty(TextSize::from(0)));

    match key.and_then(|key| key.text_ranges().next()) {
      Some(key_range) => base.cover(key_range),
      None => base,
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

  pub(crate) fn range_for_pointer(&self, pointer: &str) -> TextRange {
    let mut current = pointer;

    loop {
      if let Some(range) = self.ranges.get(current) {
        return *range;
      }

      let Some((parent, _)) = current.rsplit_once('/') else {
        return self
          .ranges
          .get("")
          .copied()
          .unwrap_or_else(|| TextRange::empty(TextSize::from(0)));
      };

      current = parent;
    }
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

    let (_, pointers) = SchemaPointer::build(&document).unwrap();

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

    let (_, pointers) = SchemaPointer::build(&document).unwrap();

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

    let (_, pointers) = SchemaPointer::build(&document).unwrap();

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
