use super::*;

macro_rules! re {
  ($pat:expr) => {
    LazyLock::new(|| Regex::new(concat!(r"^", $pat, r"$")).unwrap())
  };
}

pub(crate) static PROJECT_NAME: LazyLock<Regex> =
  re!(r"(?i)[a-z0-9](?:[a-z0-9._-]*[a-z0-9])?");

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn project_name() {
    #[track_caller]
    fn case(name: &str, expected: bool) {
      assert_eq!(PROJECT_NAME.is_match(name), expected);
    }

    case("My_Package", true);
    case("-foo", false);
    case("foo-", false);
    case("foo!", false);
  }
}
