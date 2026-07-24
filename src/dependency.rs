#[derive(Debug)]
pub struct Dependency<'a>(&'a str);

impl<'a> Dependency<'a> {
  const NAME_TERMINATORS: [char; 12] =
    [' ', '\t', '[', '(', '!', '=', '<', '>', '~', ';', '@', ','];

  #[must_use]
  pub fn name(&self) -> Option<&'a str> {
    let name = self.0.trim_start().split(Self::NAME_TERMINATORS).next()?;

    if name.is_empty() {
      return None;
    }

    Some(name)
  }

  #[must_use]
  pub fn new(value: &'a str) -> Self {
    Self(value)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn name() {
    #[track_caller]
    fn case(value: &str, expected: Option<&str>) {
      assert_eq!(Dependency::new(value).name(), expected);
    }

    case("", None);
    case("   ", None);

    case(
      "package @ https://example.com/package.tar.gz",
      Some("package"),
    );

    case("requests", Some("requests"));
    case("requests>=2.0.0", Some("requests"));
    case("requests==2.28.0", Some("requests"));
    case("requests[security]>=2.0.0", Some("requests"));
    case("requests>=2.0.0; python_version >= '3.8'", Some("requests"));
    case("  requests>=2.0.0", Some("requests"));
    case("requests >=2.0.0", Some("requests"));
    case("requests>=2.0.0,<3.0.0", Some("requests"));
    case("requests~=2.28.0", Some("requests"));
    case("requests!=2.27.0", Some("requests"));
    case("requests (>=2.0.0)", Some("requests"));
  }
}
