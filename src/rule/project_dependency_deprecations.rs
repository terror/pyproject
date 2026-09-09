use super::*;

struct DeprecatedPackage {
  extra: Option<&'static str>,
  name: &'static str,
  reason: &'static str,
}

define_rule! {
  /// Warns when `project.dependencies` includes deprecated or insecure packages.
  ///
  /// Detects known deprecated packages (e.g., `pycrypto`, `PIL`) and suggests
  /// modern alternatives.
  ProjectDependencyDeprecationsRule {
    id: "project-dependency-deprecations",
    message: "`project.dependencies` contains deprecated package",
    run(context) {
      let mut diagnostics = Vec::new();

      for dependency in context.project_dependencies().into_iter().flatten() {
        let requirement = &dependency.requirement;

        if let Some(reason) = Self::deprecated_or_insecure(requirement) {
          diagnostics.push(Diagnostic::warning(
            format!(
              "`project.dependencies` includes deprecated/insecure package `{}`: {}",
              requirement.name,
              reason.to_lowercase()
            ),
            dependency.range,
          ));
        }
      }

      diagnostics
    }
  }
}

impl ProjectDependencyDeprecationsRule {
  const DEPRECATED_OR_INSECURE_PACKAGES: &[DeprecatedPackage] = &[
    DeprecatedPackage {
      name: "pycrypto",
      extra: None,
      reason: "package is unmaintained and insecure; consider `pycryptodome`",
    },
    DeprecatedPackage {
      name: "pil",
      extra: None,
      reason: "package is deprecated; use `pillow` instead",
    },
    DeprecatedPackage {
      name: "pycryptopp",
      extra: None,
      reason: "package is unmaintained and insecure; consider `cryptography` or `pyca/cryptography`",
    },
    DeprecatedPackage {
      name: "m2crypto",
      extra: None,
      reason: "package is effectively unmaintained; consider `cryptography` instead",
    },
    DeprecatedPackage {
      name: "python-openid",
      extra: None,
      reason: "package is unmaintained; consider `python3-openid` or a maintained OpenID/OAuth library",
    },
    DeprecatedPackage {
      name: "ipaddr",
      extra: None,
      reason: "package is obsolete; use the standard library `ipaddress` module",
    },
    DeprecatedPackage {
      name: "md5",
      extra: None,
      reason: "package is obsolete and MD5 is insecure; use `hashlib` with a modern hash",
    },
    DeprecatedPackage {
      name: "sha",
      extra: None,
      reason: "package is obsolete; use `hashlib` from the standard library",
    },
    DeprecatedPackage {
      name: "imaging",
      extra: None,
      reason: "package is deprecated; use `pillow` instead",
    },
    DeprecatedPackage {
      name: "urllib2",
      extra: None,
      reason: "package is obsolete; use `urllib.request` or `requests` instead",
    },
    DeprecatedPackage {
      name: "urllib3",
      extra: Some("secure"),
      reason: "extra is deprecated; configure modern TLS via `urllib3` / `requests` directly",
    },
    DeprecatedPackage {
      name: "simplejson",
      extra: None,
      reason: "no longer needed in modern Python; use the standard library `json` module",
    },
    DeprecatedPackage {
      name: "distutils",
      extra: None,
      reason: "packaging via `distutils` is deprecated; use `setuptools` or `setuptools.build_meta`",
    },
  ];

  fn deprecated_or_insecure(
    requirement: &Requirement<VerbatimUrl>,
  ) -> Option<&'static str> {
    Self::DEPRECATED_OR_INSECURE_PACKAGES
      .iter()
      .find_map(|entry| {
        (requirement.name.as_ref() == entry.name
          && entry.extra.is_none_or(|extra| {
            requirement.extras.iter().any(|e| e.as_ref() == extra)
          }))
        .then_some(entry.reason)
      })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn deprecated_or_insecure() {
    #[track_caller]
    fn case(value: &str, expected: Option<&str>) {
      assert_eq!(
        ProjectDependencyDeprecationsRule::deprecated_or_insecure(
          &value.parse::<Requirement<VerbatimUrl>>().unwrap(),
        ),
        expected,
      );
    }

    case("foo", None);

    case(
      "pycrypto",
      Some("package is unmaintained and insecure; consider `pycryptodome`"),
    );

    case("PIL", Some("package is deprecated; use `pillow` instead"));

    case("urllib3", None);
    case("urllib3[foo]", None);

    case(
      "urllib3[SECURE]",
      Some(
        "extra is deprecated; configure modern TLS via `urllib3` / `requests` directly",
      ),
    );
  }
}
