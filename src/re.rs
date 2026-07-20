use super::*;

macro_rules! re {
  ($pat:expr) => {
    LazyLock::new(|| Regex::new($pat).unwrap())
  };
}

pub(crate) static PROJECT_NAME: LazyLock<Regex> = re!(r"[-_.]+");

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn project_name() {
    #[track_caller]
    fn case(name: &str, expected: &str) {
      assert_eq!(
        PROJECT_NAME.replace_all(name, "-").to_ascii_lowercase(),
        expected
      );
    }

    case("my-package", "my-package");
    case("My__Package.Name-Tool", "my-package-name-tool");
    case("_my_package_", "-my-package-");
  }
}
