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
