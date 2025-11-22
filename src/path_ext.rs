use super::*;

pub(crate) trait PathExt {
  fn rooted(&self) -> bool;
}

impl PathExt for &Path {
  fn rooted(&self) -> bool {
    self.has_root()
      || self
        .components()
        .any(|component| matches!(component, Component::Prefix(_)))
  }
}
