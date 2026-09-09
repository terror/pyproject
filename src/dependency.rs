use super::*;

#[derive(Debug)]
pub(crate) enum DependencyError {
  InvalidRequirement {
    error: Pep508Error,
    range: lsp::Range,
  },
  NotArray(lsp::Range),
  NotString(lsp::Range),
}

#[derive(Debug)]
pub(crate) struct Dependency {
  pub(crate) range: lsp::Range,
  pub(crate) requirement: Requirement<VerbatimUrl>,
  value: String,
}

impl Dependency {
  const NAME_TERMINATORS: [char; 12] =
    [' ', '\t', '[', '(', '!', '=', '<', '>', '~', ';', '@', ','];

  pub(crate) fn from_array(
    node: &Node,
    context: &RuleContext,
  ) -> Vec<Result<Self, DependencyError>> {
    let Some(array) = node.as_array() else {
      return vec![Err(DependencyError::NotArray(
        node.span(context.content()),
      ))];
    };

    array
      .items()
      .read()
      .iter()
      .map(|item| Self::new(item, context))
      .collect()
  }

  fn new(item: &Node, context: &RuleContext) -> Result<Self, DependencyError> {
    let range = item.span(context.content());

    let string = item.as_str().ok_or(DependencyError::NotString(range))?;

    let value = string.value();

    let requirement = value
      .parse()
      .map_err(|error| DependencyError::InvalidRequirement { error, range })?;

    let value = value.to_string();

    Ok(Self {
      range,
      requirement,
      value,
    })
  }

  pub(crate) fn raw_name(&self) -> &str {
    self
      .value
      .trim_start()
      .split(Self::NAME_TERMINATORS)
      .next()
      .unwrap()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn raw_name() {
    #[track_caller]
    fn case(value: &str, expected: Option<&str>) {
      let document = Document::from(format!("foo = {value:?}").as_str());

      let context = RuleContext::new(&document);

      assert_eq!(
        Dependency::new(&context.get("foo").unwrap(), &context)
          .ok()
          .as_ref()
          .map(Dependency::raw_name),
        expected,
      );
    }

    case("", None);
    case("   ", None);

    case("foo @ https://example.com/foo.tar.gz", Some("foo"));

    case("foo", Some("foo"));
    case("Foo", Some("Foo"));
    case("foo>=2.0.0", Some("foo"));
    case("foo==2.28.0", Some("foo"));
    case("foo[bar]>=2.0.0", Some("foo"));
    case("foo>=2.0.0; python_version >= '3.8'", Some("foo"));
    case("  foo>=2.0.0", Some("foo"));
    case("foo >=2.0.0", Some("foo"));
    case("foo>=2.0.0,<3.0.0", Some("foo"));
    case("foo~=2.28.0", Some("foo"));
    case("foo!=2.27.0", Some("foo"));
    case("foo (>=2.0.0)", Some("foo"));
  }
}
