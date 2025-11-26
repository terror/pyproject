use super::*;

struct DeprecatedPackage {
  extra: Option<&'static str>,
  name: &'static str,
  reason: &'static str,
}

pub(crate) struct ProjectDependencyDeprecationsRule;

impl Rule for ProjectDependencyDeprecationsRule {
  fn display(&self) -> &'static str {
    "`project.dependencies` contains deprecated package"
  }

  fn id(&self) -> &'static str {
    "project-dependency-deprecations"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(dependencies) = context.get("project.dependencies") else {
      return Vec::new();
    };

    let Some(array) = dependencies.as_array() else {
      return Vec::new();
    };

    let document = context.document();

    let mut diagnostics = Vec::new();

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        continue;
      };

      let Ok(requirement) =
        Requirement::<VerbatimUrl>::from_str(string.value())
      else {
        continue;
      };

      if let Some(reason) = Self::deprecated_or_insecure(
        requirement.name.as_ref(),
        &requirement.extras,
      ) {
        diagnostics.push(Diagnostic::warning(
          format!(
            "`project.dependencies` includes deprecated/insecure package `{}`: {}",
            requirement.name,
            reason.to_lowercase()
          ),
          item.span(&document.content),
        ));
      }
    }

    diagnostics
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
    name: &str,
    extras: &[ExtraName],
  ) -> Option<&'static str> {
    let Ok(package) = PackageName::from_str(name) else {
      return None;
    };

    let normalized = package.as_ref();

    Self::DEPRECATED_OR_INSECURE_PACKAGES
      .iter()
      .find_map(|entry| {
        (normalized == entry.name
          && entry
            .extra
            .is_none_or(|extra| extras.iter().any(|e| e.as_ref() == extra)))
        .then_some(entry.reason)
      })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn deprecated_or_insecure_pycrypto() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure(
        "pycrypto",
        &[],
      ),
      Some("package is unmaintained and insecure; consider `pycryptodome`")
    );
  }

  #[test]
  fn deprecated_or_insecure_pil() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure("pil", &[]),
      Some("package is deprecated; use `pillow` instead")
    );
  }

  #[test]
  fn deprecated_or_insecure_pil_uppercase() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure("PIL", &[]),
      Some("package is deprecated; use `pillow` instead")
    );
  }

  #[test]
  fn deprecated_or_insecure_safe_package() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure(
        "requests",
        &[]
      ),
      None
    );
  }

  #[test]
  fn deprecated_or_insecure_pillow() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure("pillow", &[]),
      None
    );
  }

  #[test]
  fn deprecated_or_insecure_pycryptodome() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure(
        "pycryptodome",
        &[]
      ),
      None
    );
  }

  #[test]
  fn deprecated_or_insecure_invalid_package_name() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure(
        "!!!invalid!!!",
        &[]
      ),
      None
    );
  }

  #[test]
  fn deprecated_or_insecure_m2crypto_uppercase() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure(
        "M2Crypto",
        &[]
      ),
      Some(
        "package is effectively unmaintained; consider `cryptography` instead"
      )
    );
  }

  #[test]
  fn deprecated_or_insecure_urllib3_secure_extra() {
    let extra = ExtraName::from_str("secure").unwrap();

    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure(
        "urllib3",
        &[extra]
      ),
      Some(
        "extra is deprecated; configure modern TLS via `urllib3` / `requests` directly"
      )
    );
  }

  #[test]
  fn deprecated_or_insecure_urllib3_without_extra() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure("urllib3", &[]),
      None
    );
  }
}
