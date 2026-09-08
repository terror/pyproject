use super::*;

pub(crate) trait VersionSpecifiersExt {
  fn has_upper_bound(&self) -> bool;
}

impl VersionSpecifiersExt for VersionSpecifiers {
  fn has_upper_bound(&self) -> bool {
    self.iter().any(|specifier| {
      matches!(
        specifier.operator(),
        Operator::Equal
          | Operator::EqualStar
          | Operator::ExactEqual
          | Operator::LessThan
          | Operator::LessThanEqual
          | Operator::TildeEqual
      )
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn has_upper_bound() {
    #[track_caller]
    fn case(value: &str, expected: bool) {
      assert_eq!(
        value
          .parse::<VersionSpecifiers>()
          .unwrap()
          .has_upper_bound(),
        expected,
      );
    }

    case("", false);
    case("==1", true);
    case("==1.*", true);
    case("===1", true);
    case("<1", true);
    case("<=1", true);
    case("~=1.0", true);
    case("!=1", false);
    case("!=1.*", false);
    case(">1", false);
    case(">=1", false);
    case(">=1,!=2.*", false);
    case(">=1,!=2.*,<3", true);
  }
}
