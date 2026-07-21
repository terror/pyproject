use super::*;

#[derive(Debug)]
pub(crate) struct Resolver<'a> {
  document: &'a Document,
}

impl<'a> Resolver<'a> {
  pub(crate) fn new(document: &'a Document) -> Self {
    Self { document }
  }

  pub(crate) fn resolve_hover(
    &self,
    position: lsp::Position,
  ) -> Option<lsp::Hover> {
    let (instance, pointers) = SchemaPointer::build(self.document).ok()?;

    let pointer = pointers.pointer_for_position(position)?;

    let validator = SchemaRule::validator(&self.document.config).ok()?;

    let evaluation = validator.evaluate(&instance);

    let description = evaluation
      .iter_annotations()
      .filter(|entry| entry.instance_location.as_str() == pointer)
      .find_map(|entry| {
        entry
          .annotations
          .value()
          .get("description")
          .and_then(Value::as_str)
      })?;

    Some(lsp::Hover {
      contents: lsp::HoverContents::Markup(lsp::MarkupContent {
        kind: lsp::MarkupKind::Markdown,
        value: description.to_string(),
      }),
      range: Some(
        pointers
          .range_for_pointer(&pointer)
          .span(&self.document.content),
      ),
    })
  }
}

#[cfg(test)]
mod tests {
  use {super::*, indoc::indoc, pretty_assertions::assert_eq};

  #[test]
  fn resolve_hover_returns_schema_description() {
    let document = Document::from(indoc! {
      r#"
      [tool.poetry]
      name = "demo"
      "#
    });

    let hover = Resolver::new(&document)
      .resolve_hover(lsp::Position::new(1, 1))
      .unwrap();

    assert_eq!(
      hover,
      lsp::Hover {
        contents: lsp::HoverContents::Markup(lsp::MarkupContent {
          kind: lsp::MarkupKind::Markdown,
          value: "Package name.".to_string(),
        }),
        range: Some((1, 0, 1, 13).range()),
      }
    );
  }
}
