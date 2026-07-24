use super::*;

define_rule! {
  /// Validates `project.import-names` and `project.import-namespaces` configuration.
  ///
  /// Ensures entries are strings, checks for duplicates across both fields,
  /// and verifies that all parent namespaces are declared for nested names.
  ProjectImportNamesRule {
    id: "project-import-names",
    message: "invalid `project.import-names` / `project.import-namespaces` configuration",
    run(context) {
      let content = context.content();

      let mut diagnostics = Vec::new();

      let mut entries = Vec::new();

      if let Some(import_names) = context.get("project.import-names") {
        Self::collect_entries(
          content,
          "project.import-names",
          import_names,
          true,
          &mut diagnostics,
          &mut entries,
        );
      }

      if let Some(import_namespaces) = context.get("project.import-namespaces")
      {
        Self::collect_entries(
          content,
          "project.import-namespaces",
          import_namespaces,
          false,
          &mut diagnostics,
          &mut entries,
        );
      }

      if entries.is_empty() {
        return diagnostics;
      }

      let mut seen = HashSet::new();

      for (name, node) in &entries {
        if !seen.insert(name.clone()) {
          diagnostics.push(Self::duplicate_name_diagnostic(
            content, node, name,
          ));
        }
      }

      let available: HashSet<String> =
        entries.iter().map(|(name, _)| name.clone()).collect();

      for (name, node) in &entries {
        for parent in Self::parent_names(name) {
          if !available.contains(&parent) {
            diagnostics.push(Self::missing_parent_diagnostic(
              content, node, name, &parent,
            ));

            break;
          }
        }
      }

      diagnostics
    }
  }
}

impl ProjectImportNamesRule {
  fn collect_entries(
    content: &Rope,
    field: &'static str,
    node: Node,
    allow_empty_name: bool,
    diagnostics: &mut Vec<Diagnostic>,
    entries: &mut Vec<(String, Node)>,
  ) {
    let Some(array) = node.as_array() else {
      diagnostics.push(Diagnostic::error(
        format!("`{field}` must be an array of strings"),
        node.span(content),
      ));

      return;
    };

    if array.items().read().is_empty() && !allow_empty_name {
      diagnostics.push(Diagnostic::error(
        format!("`{field}` must not be an empty array"),
        node.span(content),
      ));
    }

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        diagnostics.push(Diagnostic::error(
          format!("`{field}` items must be strings"),
          item.span(content),
        ));

        continue;
      };

      let parsed = match Self::parse_name(string.value(), allow_empty_name) {
        Ok(parsed) => parsed,
        Err(ImportNameError::InvalidIdentifier) => {
          diagnostics.push(Diagnostic::error(
            format!(
              "`{field}` item `{}` must be a valid dotted Python identifier",
              string.value()
            ),
            item.span(content),
          ));

          continue;
        }
        Err(ImportNameError::Keyword(keyword)) => {
          diagnostics.push(Diagnostic::error(
            format!(
              "`{field}` item `{}` contains Python keyword `{keyword}`",
              string.value()
            ),
            item.span(content),
          ));

          continue;
        }
        Err(ImportNameError::InvalidSuffix) => {
          diagnostics.push(Diagnostic::error(
            format!(
              "`{field}` item `{}` has an invalid suffix; only `; private` is allowed",
              string.value()
            ),
            item.span(content),
          ));

          continue;
        }
      };

      entries.push((parsed.name.to_string(), item.clone()));
    }
  }

  fn duplicate_name_diagnostic(
    content: &Rope,
    node: &Node,
    name: &str,
  ) -> Diagnostic {
    Diagnostic::error(
      format!(
        "duplicated names are not allowed in `project.import-names`/`project.import-namespaces` (found `{name}`)"
      ),
      node.span(content),
    )
  }

  fn is_identifier(value: &str) -> bool {
    let mut characters = value.chars();

    let Some(first) = characters.next() else {
      return false;
    };

    (unicode_ident::is_xid_start(first) || first == '_')
      && characters.all(unicode_ident::is_xid_continue)
  }

  fn is_python_keyword(value: &str) -> bool {
    matches!(
      value,
      "False"
        | "None"
        | "True"
        | "and"
        | "as"
        | "assert"
        | "async"
        | "await"
        | "break"
        | "class"
        | "continue"
        | "def"
        | "del"
        | "elif"
        | "else"
        | "except"
        | "finally"
        | "for"
        | "from"
        | "global"
        | "if"
        | "import"
        | "in"
        | "is"
        | "lambda"
        | "nonlocal"
        | "not"
        | "or"
        | "pass"
        | "raise"
        | "return"
        | "try"
        | "while"
        | "with"
        | "yield"
    )
  }

  fn missing_parent_diagnostic(
    content: &Rope,
    node: &Node,
    name: &str,
    parent: &str,
  ) -> Diagnostic {
    Diagnostic::error(
      format!(
        "`{name}` is missing parent namespace `{parent}`; all parents must be listed in `project.import-names`/`project.import-namespaces`"
      ),
      node.span(content),
    )
  }

  fn parent_names(name: &str) -> Vec<String> {
    let mut parents = Vec::new();

    let mut current = String::new();

    let mut segments = name.split('.').peekable();

    while let Some(segment) = segments.next() {
      if segments.peek().is_none() {
        break;
      }

      if !current.is_empty() {
        current.push('.');
      }

      current.push_str(segment);

      if !current.is_empty() {
        parents.push(current.clone());
      }
    }

    parents
  }

  fn parse_name(
    raw: &str,
    allow_empty_name: bool,
  ) -> Result<ParsedImportName<'_>, ImportNameError<'_>> {
    let (name, is_private) = match raw.split_once(';') {
      Some((name, suffix)) => {
        let name = name.trim_end_matches(char::is_whitespace);
        let suffix = suffix.trim_start_matches(char::is_whitespace);

        if suffix != "private" {
          return Err(ImportNameError::InvalidSuffix);
        }

        (name, true)
      }
      None => (raw, false),
    };

    if name.is_empty() {
      return if allow_empty_name && !is_private {
        Ok(ParsedImportName { is_private, name })
      } else {
        Err(ImportNameError::InvalidIdentifier)
      };
    }

    for component in name.split('.') {
      if Self::is_python_keyword(component) {
        return Err(ImportNameError::Keyword(component));
      }

      if !Self::is_identifier(component) {
        return Err(ImportNameError::InvalidIdentifier);
      }
    }

    Ok(ParsedImportName { is_private, name })
  }
}

#[derive(Debug, Eq, PartialEq)]
struct ParsedImportName<'a> {
  is_private: bool,
  name: &'a str,
}

#[derive(Debug, Eq, PartialEq)]
enum ImportNameError<'a> {
  InvalidIdentifier,
  InvalidSuffix,
  Keyword(&'a str),
}

#[cfg(test)]
mod tests {
  use {super::*, pretty_assertions::assert_eq};

  #[test]
  fn parsing() {
    #[track_caller]
    fn case(
      raw: &str,
      allow_empty_name: bool,
      expected: Result<ParsedImportName<'_>, ImportNameError<'_>>,
    ) {
      assert_eq!(
        ProjectImportNamesRule::parse_name(raw, allow_empty_name),
        expected
      );
    }

    for value in ["foo", "foo.bar", "_foo._bar", "\u{00e9}.\u{03b2}", "x"] {
      case(
        value,
        false,
        Ok(ParsedImportName {
          is_private: false,
          name: value,
        }),
      );
    }

    for value in [
      "foo;private",
      "foo; private",
      "foo ;private",
      "foo \t;\tprivate",
    ] {
      case(
        value,
        false,
        Ok(ParsedImportName {
          is_private: true,
          name: "foo",
        }),
      );
    }

    case(
      "",
      true,
      Ok(ParsedImportName {
        is_private: false,
        name: "",
      }),
    );
    case("", false, Err(ImportNameError::InvalidIdentifier));

    for value in ["1foo", "foo bar", "foo..bar", "foo.", "foo!", ""] {
      case(value, false, Err(ImportNameError::InvalidIdentifier));
    }

    case("class", false, Err(ImportNameError::Keyword("class")));
    case("foo.class", false, Err(ImportNameError::Keyword("class")));

    for value in [
      "foo; public",
      "foo; python_version >= '3.11'",
      "foo; private extra",
      "foo ; private ",
    ] {
      case(value, false, Err(ImportNameError::InvalidSuffix));
    }
  }

  #[test]
  fn parent_names_builds_all_namespaces() {
    assert_eq!(
      ProjectImportNamesRule::parent_names("foo.bar.baz"),
      vec!["foo".to_string(), "foo.bar".to_string()]
    );

    assert!(ProjectImportNamesRule::parent_names("foo").is_empty());
  }
}
