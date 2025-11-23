#[derive(Debug)]
pub(crate) struct Schema {
  pub(crate) contents: &'static str,
  pub(crate) tool: Option<&'static str>,
  pub(crate) url: &'static str,
}
