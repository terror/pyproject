use super::*;

#[derive(Debug)]
pub(crate) enum DependencyError {
  InvalidRequirement {
    error: Pep508Error,
    range: lsp::Range,
    value: String,
  },
  NotArray(lsp::Range),
  NotString(lsp::Range),
}

#[derive(Debug)]
pub(crate) struct ParsedDependency {
  pub(crate) range: lsp::Range,
  pub(crate) requirement: Requirement<VerbatimUrl>,
  pub(crate) value: String,
}

impl ParsedDependency {
  pub(crate) fn new(
    item: &Node,
    context: &RuleContext,
  ) -> Result<Self, DependencyError> {
    let range = item.span(context.content());

    let string = item.as_str().ok_or(DependencyError::NotString(range))?;

    let value = string.value();

    let requirement =
      value
        .parse()
        .map_err(|error| DependencyError::InvalidRequirement {
          error,
          range,
          value: value.to_string(),
        })?;

    let value = value.to_string();

    Ok(Self {
      range,
      requirement,
      value,
    })
  }
}
