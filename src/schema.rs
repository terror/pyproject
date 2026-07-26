#[derive(Debug)]
pub struct Schema {
  pub contents: &'static str,
  pub tool: Option<&'static str>,
  pub(crate) url: &'static str,
}
