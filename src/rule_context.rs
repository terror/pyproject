use super::*;

pub(crate) struct RuleContext<'a> {
  document: &'a Document,
}

impl<'a> RuleContext<'a> {
  pub(crate) fn document(&self) -> &Document {
    self.document
  }

  pub(crate) fn get(&self, path: &str) -> Option<Node> {
    let mut current = self.document.tree.clone().into_dom();

    if path.is_empty() {
      return Some(current);
    }

    for key in path.split('.') {
      if key.is_empty() {
        return None;
      }

      let Ok(next) = current.try_get(key) else {
        return None;
      };

      current = next;
    }

    Some(current)
  }

  pub(crate) fn new(document: &'a Document) -> Self {
    Self { document }
  }

  pub(crate) fn project(&self) -> Option<Node> {
    self.get("project")
  }

  pub(crate) fn tree(&self) -> &Parse {
    &self.document.tree
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn get_returns_root_for_empty_path() {
    let document = Document::from(indoc! {
      r#"
      [project]
      name = "demo"
      "#
    });

    let context = RuleContext::new(&document);

    let root = context.get("").unwrap();

    match root {
      Node::Table(table) => assert!(table.get("project").is_some()),
      other => panic!("expected document root to be a table, got {other:?}"),
    }
  }

  #[test]
  fn get_returns_nested_value() {
    let document = Document::from(indoc! {
      r#"
      [project]
      name = "demo"
      description = "example"
      "#
    });

    let context = RuleContext::new(&document);

    let name = context.get("project.name").unwrap();

    match name {
      Node::Str(value) => assert_eq!(value.value(), "demo"),
      other => panic!("expected project.name to be a string, got {other:?}"),
    }
  }

  #[test]
  fn get_rejects_invalid_paths() {
    let document = Document::from(indoc! {
      r#"
      [project]
      name = "demo"
      "#
    });

    let context = RuleContext::new(&document);

    let cases = &[
      "nonexistent",
      "project.",
      "project.name.extra",
      "project.nonexistent",
    ];

    for case in cases {
      assert!(
        context.get(case).is_none(),
        "expected path '{case}' to be invalid"
      );
    }
  }
}
